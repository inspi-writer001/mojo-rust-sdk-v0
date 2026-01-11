use anyhow::{ensure, Result};
use bytemuck::{bytes_of, from_bytes, Pod, Zeroable};
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_signer::Signer;

use crate::{
    client::{
        RpcLayer, RpcType, WorldClient, BASE_LAYER_RPC_DEVNET, BASE_LAYER_RPC_MAINNET,
        ER_LAYER_RPC_DEVNET, ER_LAYER_RPC_MAINNET,
    },
    instructions::{create_world_ix, delegate_account_ix, write_to_world_ix},
    pda::{find_world_pda, world_seed_hash},
    profile::{
        create_mpl_core_asset_ix, fetch_metadata_from_uri, fetch_mpl_core_asset, load_image_data,
        update_mpl_core_asset_ix, validate_image, ArweaveUploader, ImageSource, ProfilePicture,
        ProfilePictureData,
    },
};

pub trait MojoState: Pod + Zeroable + Copy {}

impl<T> MojoState for T where T: Pod + Zeroable + Copy {}

// #[repr(C)]
// #[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
// pub struct World {
//     pub creator: [u8; 32],
//     pub seed: [u8; 32],
//     pub world_address: [u8; 32],
// }

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct WorldData {
    pub creator: [u8; 32],
    pub seed: [u8; 32],
    pub world_address: [u8; 32],
}

pub struct World {
    pub data: WorldData,
    pub network: RpcType,
}

impl World {
    pub fn create_world(network: RpcType, payer: &impl Signer, name: &str) -> Result<Self> {
        let (world_pda, _) = find_world_pda(&payer.pubkey(), name);
        let seed_hash = world_seed_hash(&payer.pubkey(), name);

        let world_data = WorldData {
            creator: payer.pubkey().to_bytes(),
            seed: seed_hash,
            world_address: world_pda.to_bytes(),
        };

        let ix = create_world_ix(payer.pubkey(), world_pda, seed_hash, bytes_of(&world_data));

        WorldClient::new(&network).send_ixs(payer, vec![ix], RpcLayer::BaseLayer)?;

        Ok(Self {
            data: world_data,
            network,
        })
    }

    pub fn create_state<T: MojoState>(
        &self,
        payer: &impl Signer,
        name: &str,
        initial_state: &T,
    ) -> Result<Pubkey> {
        let (state_pda, _) = find_world_pda(&payer.pubkey(), name);
        let seed_hash = world_seed_hash(&payer.pubkey(), name);
        let ix = create_world_ix(
            payer.pubkey(),
            state_pda,
            seed_hash,
            bytes_of(initial_state),
        );

        let delegate_ix = delegate_account_ix(
            payer.pubkey(),
            state_pda,
            seed_hash,
            bytes_of(initial_state),
        );

        WorldClient::new(&self.network).send_ixs(payer, vec![ix], RpcLayer::BaseLayer)?;
        WorldClient::new(&self.network).send_ixs(payer, vec![delegate_ix], RpcLayer::BaseLayer)?;
        Ok(state_pda)
    }

    pub fn write_state<T: MojoState>(
        &self,
        payer: &impl Signer,
        name: &str,
        new_state: &T,
    ) -> Result<Signature> {
        let (world_pda, _) = find_world_pda(&payer.pubkey(), name);
        let seed_hash = world_seed_hash(&payer.pubkey(), name);
        let ix = write_to_world_ix(payer.pubkey(), world_pda, seed_hash, bytes_of(new_state));

        let tx = WorldClient::new(&self.network).send_ixs(payer, vec![ix], RpcLayer::Ephemeral)?;
        Ok(tx)
    }

    pub fn read_state<T: MojoState>(&self, owner: &Pubkey, name: &str) -> Result<T> {
        let (world_pda, _) = find_world_pda(owner, name);

        let rpc = match self.network {
            RpcType::Devnet => RpcClient::new(ER_LAYER_RPC_DEVNET),

            RpcType::Mainnet => RpcClient::new(ER_LAYER_RPC_MAINNET),
        };
        let data = rpc.get_account_data(&world_pda)?;
        let required_len = core::mem::size_of::<T>();
        ensure!(
            data.len() >= required_len,
            "account data length {} smaller than expected {}",
            data.len(),
            required_len
        );

        let state = *from_bytes::<T>(&data[..required_len]);
        Ok(state)
    }

    pub async fn create_profile_picture(
        &self,
        user: &impl Signer,
        payer: Option<&impl Signer>,
        image_source: ImageSource,
        name: &str,
        description: Option<&str>,
        uploader: Option<ArweaveUploader>,
    ) -> Result<ProfilePicture> {
        let image_data = load_image_data(&image_source).await?;
        validate_image(&image_data)?;

        let uploader = uploader.unwrap_or_default();
        let image_tx_id = uploader.upload(&image_data, Some("image/png")).await?;
        let image_uri = uploader.uri_from_tx_id(&image_tx_id);

        let metadata = crate::profile::Metadata::new(name, description, &image_uri);

        let metadata_json = serde_json::to_vec(&metadata)
            .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))?;
        let metadata_tx_id = uploader
            .upload(&metadata_json, Some("application/json"))
            .await?;
        let metadata_uri = uploader.uri_from_tx_id(&metadata_tx_id);

        let asset_keypair = Keypair::new();
        let asset_pubkey = asset_keypair.pubkey();

        let effective_payer = payer.map(|p| p.pubkey()).unwrap_or_else(|| user.pubkey());
        let create_ix = create_mpl_core_asset_ix(
            &asset_pubkey,
            user.pubkey(),
            effective_payer,
            name,
            &metadata_uri,
        )?;

        let signers: Vec<&dyn Signer> = vec![user as &dyn Signer];

        if let Some(p) = payer {
            WorldClient::new(&self.network).send_ixs_with_payer(
                p,
                &signers,
                vec![create_ix],
                RpcLayer::BaseLayer,
            )?;
        } else {
            WorldClient::new(&self.network).send_ixs_with_payer(
                user,
                &signers,
                vec![create_ix],
                RpcLayer::BaseLayer,
            )?;
        }

        Ok(ProfilePicture {
            asset: asset_pubkey,
            collection: None,
            owner: user.pubkey(),
        })
    }

    pub async fn get_profile_picture(&self, asset: &Pubkey) -> Result<ProfilePictureData> {
        let rpc = match self.network {
            RpcType::Devnet => RpcClient::new(BASE_LAYER_RPC_DEVNET),
            RpcType::Mainnet => RpcClient::new(BASE_LAYER_RPC_MAINNET),
        };

        let mpl_asset = fetch_mpl_core_asset(&rpc, asset)?;

        let owner = mpl_asset.base.owner;
        let collection = None;
        let metadata_uri = mpl_asset.base.uri;

        let metadata = fetch_metadata_from_uri(&metadata_uri).await?;

        Ok(ProfilePictureData {
            asset: *asset,
            collection,
            owner,
            name: metadata.name,
            description: metadata.description,
            image_uri: metadata.image,
            metadata_uri,
        })
    }

    pub async fn modify_profile_picture(
        &self,
        user: &impl Signer,
        payer: Option<&impl Signer>,
        asset: &Pubkey,
        new_image: Option<ImageSource>,
        new_name: Option<&str>,
        new_description: Option<&str>,
        uploader: Option<ArweaveUploader>,
    ) -> Result<ProfilePictureData> {
        let uploader = uploader.unwrap_or_default();

        let rpc = match self.network {
            RpcType::Devnet => RpcClient::new(BASE_LAYER_RPC_DEVNET),
            RpcType::Mainnet => RpcClient::new(BASE_LAYER_RPC_MAINNET),
        };

        let mpl_asset = fetch_mpl_core_asset(&rpc, asset)?;

        if mpl_asset.base.owner != user.pubkey() {
            return Err(crate::error::WorldError::NotAuthorized(format!("asset {}", asset)).into());
        }

        let current_metadata_uri = mpl_asset.base.uri;
        let current_metadata = fetch_metadata_from_uri(&current_metadata_uri).await?;

        let effective_name = new_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| current_metadata.name.clone());

        let effective_description = new_description
            .map(|s| s.to_string())
            .unwrap_or_else(|| current_metadata.description.clone());

        let effective_image_uri = if let Some(image_source) = new_image {
            let image_data = load_image_data(&image_source).await?;
            validate_image(&image_data)?;

            let image_tx_id = uploader.upload(&image_data, Some("image/png")).await?;
            uploader.uri_from_tx_id(&image_tx_id)
        } else {
            current_metadata.image.clone()
        };

        let new_metadata = crate::profile::Metadata::new(
            &effective_name,
            Some(&effective_description),
            &effective_image_uri,
        );

        let metadata_json = serde_json::to_vec(&new_metadata)
            .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))?;
        let metadata_tx_id = uploader
            .upload(&metadata_json, Some("application/json"))
            .await?;
        let new_metadata_uri = uploader.uri_from_tx_id(&metadata_tx_id);

        let effective_payer = payer.map(|p| p.pubkey()).unwrap_or_else(|| user.pubkey());

        let new_name_for_onchain = new_name.map(|s| s.to_string());
        let update_ix = update_mpl_core_asset_ix(
            asset,
            user.pubkey(),
            effective_payer,
            new_name_for_onchain,
            Some(new_metadata_uri.clone()),
        )?;

        let signers: Vec<&dyn Signer> = vec![user as &dyn Signer];

        if let Some(p) = payer {
            WorldClient::new(&self.network).send_ixs_with_payer(
                p,
                &signers,
                vec![update_ix],
                RpcLayer::BaseLayer,
            )?;
        } else {
            WorldClient::new(&self.network).send_ixs_with_payer(
                user,
                &signers,
                vec![update_ix],
                RpcLayer::BaseLayer,
            )?;
        }

        self.get_profile_picture(asset).await
    }
}

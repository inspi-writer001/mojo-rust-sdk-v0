use anyhow::{ensure, Result};
use bytemuck::{bytes_of, from_bytes, Pod, Zeroable};
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_signer::Signer;

use crate::{
    badges::{
        create_mpl_core_badge_collection_ix, create_mpl_core_badge_ix, update_mpl_core_badge_ix,
        BadgeCollection, BadgeCollectionMetadata, BadgeMetadata, BadgeMint, BadgeTemplate,
        QualifyingAction,
    },
    client::{
        RpcLayer, RpcType, WorldClient, BASE_LAYER_RPC_DEVNET, BASE_LAYER_RPC_MAINNET,
        ER_LAYER_RPC_DEVNET, ER_LAYER_RPC_MAINNET,
    },
    instructions::{create_world_ix, delegate_account_ix, write_to_world_ix},
    pda::{find_world_pda, world_seed_hash},
    profile::{
        create_mpl_core_asset_ix, fetch_metadata_from_uri, fetch_mpl_core_asset, ArweaveUploader,
        ImageSource, ProfilePicture, ProfilePictureData,
    },
    utils::{send_with_signers, upload_metadata},
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
        let metadata_uri = upload_metadata(
            image_source,
            |image_uri| crate::profile::Metadata::new(name, description, &image_uri),
            uploader,
        )
        .await?;

        let asset_keypair = Keypair::new();
        let asset_pubkey = asset_keypair.pubkey();

        let effective_payer = payer.map(|p| p.pubkey()).unwrap_or_else(|| user.pubkey());
        let create_ix =
            create_mpl_core_asset_ix(&asset_pubkey, user.pubkey(), effective_payer, name, &metadata_uri)?;

        let signers: Vec<&dyn Signer> = vec![user as &dyn Signer, &asset_keypair];
        let payer_signer: &dyn Signer = payer.map(|p| p as &dyn Signer).unwrap_or(user as &dyn Signer);
        send_with_signers(self.network, payer_signer, &signers, create_ix, RpcLayer::BaseLayer)?;

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

    pub async fn create_badge_collection(
        &self,
        collection: &Keypair,
        update_authority: Option<&impl Signer>,
        payer: Option<&impl Signer>,
        image_source: ImageSource,
        name: &str,
        description: Option<&str>,
        uploader: Option<ArweaveUploader>,
    ) -> Result<BadgeCollection> {
        let metadata_uri = upload_metadata(
            image_source,
            |image_uri| BadgeCollectionMetadata::new(name, description, &image_uri),
            uploader,
        )
        .await?;

        let collection_pubkey = collection.pubkey();

        let authority_pubkey = update_authority
            .map(|a| a.pubkey())
            .unwrap_or(collection_pubkey);

        let effective_payer = payer
            .map(|p| p.pubkey())
            .unwrap_or(authority_pubkey);
        let create_ix = create_mpl_core_badge_collection_ix(
            &collection_pubkey,
            effective_payer,
            authority_pubkey,
            name,
            &metadata_uri,
        )?;

        let signers: Vec<&dyn Signer> = vec![collection as &dyn Signer];
        let payer_signer: &dyn Signer = payer
            .map(|p| p as &dyn Signer)
            .or_else(|| update_authority.map(|a| a as &dyn Signer))
            .unwrap_or(collection as &dyn Signer);
        send_with_signers(self.network, payer_signer, &signers, create_ix, RpcLayer::BaseLayer)?;

        Ok(BadgeCollection {
            collection: collection_pubkey,
            update_authority: authority_pubkey,
        })
    }

    pub async fn save_to_badge_collection(
        &self,
        authority: &impl Signer,
        payer: Option<&impl Signer>,
        collection: &Pubkey,
        qualifying_action: QualifyingAction,
        image_source: ImageSource,
        name: &str,
        description: Option<&str>,
        uploader: Option<ArweaveUploader>,
    ) -> Result<BadgeTemplate> {
        let metadata_uri = upload_metadata(
            image_source,
            |image_uri| BadgeMetadata::new(name, description, &image_uri, qualifying_action.clone()),
            uploader,
        )
        .await?;

        let badge_keypair = Keypair::new();
        let badge_pubkey = badge_keypair.pubkey();

        let effective_payer = payer
            .map(|p| p.pubkey())
            .unwrap_or_else(|| authority.pubkey());
        let create_ix = create_mpl_core_badge_ix(
            &badge_pubkey,
            *collection,
            authority.pubkey(),
            authority.pubkey(),
            effective_payer,
            name,
            &metadata_uri,
        )?;

        let signers: Vec<&dyn Signer> = vec![authority as &dyn Signer, &badge_keypair];
        let payer_signer: &dyn Signer =
            payer.map(|p| p as &dyn Signer).unwrap_or(authority as &dyn Signer);
        send_with_signers(self.network, payer_signer, &signers, create_ix, RpcLayer::BaseLayer)?;

        Ok(BadgeTemplate {
            asset: badge_pubkey,
            collection: *collection,
            name: name.to_string(),
            uri: metadata_uri,
            qualifying_action,
        })
    }

    pub fn modify_badge(
        &self,
        authority: &impl Signer,
        payer: Option<&impl Signer>,
        badge: &Pubkey,
        collection: Option<&Pubkey>,
        new_name: Option<&str>,
        new_uri: Option<&str>,
    ) -> Result<Signature> {
        let effective_payer = payer
            .map(|p| p.pubkey())
            .unwrap_or_else(|| authority.pubkey());

        let update_ix = update_mpl_core_badge_ix(
            badge,
            collection.copied(),
            effective_payer,
            authority.pubkey(),
            new_name,
            new_uri,
        );

        let signers: Vec<&dyn Signer> = vec![authority as &dyn Signer];
        let payer_signer: &dyn Signer =
            payer.map(|p| p as &dyn Signer).unwrap_or(authority as &dyn Signer);
        let tx = send_with_signers(self.network, payer_signer, &signers, update_ix, RpcLayer::BaseLayer)?;
        Ok(tx)
    }

    pub fn claim_badge(
        &self,
        authority: &impl Signer,
        payer: Option<&impl Signer>,
        template: &BadgeTemplate,
        new_owner: Pubkey,
    ) -> Result<BadgeMint> {
        let effective_payer = payer
            .map(|p| p.pubkey())
            .unwrap_or_else(|| authority.pubkey());

        let badge_keypair = Keypair::new();
        let badge_pubkey = badge_keypair.pubkey();

        let create_ix = create_mpl_core_badge_ix(
            &badge_pubkey,
            template.collection,
            authority.pubkey(),
            new_owner,
            effective_payer,
            &template.name,
            &template.uri,
        )?;

        let signers: Vec<&dyn Signer> = vec![authority as &dyn Signer, &badge_keypair];

        let payer_signer: &dyn Signer =
            payer.map(|p| p as &dyn Signer).unwrap_or(authority as &dyn Signer);
        let tx = send_with_signers(self.network, payer_signer, &signers, create_ix, RpcLayer::BaseLayer)?;

        Ok(BadgeMint {
            signature: tx,
            badge: badge_pubkey,
            owner: new_owner,
        })
    }

}

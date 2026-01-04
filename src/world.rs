use anyhow::{ensure, Result};
use bytemuck::{bytes_of, from_bytes, Pod, Zeroable};
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_signer::Signer;

use crate::{
    client::{RpcLayer, RpcType, WorldClient, ER_LAYER_RPC_DEVNET, ER_LAYER_RPC_MAINNET},
    instructions::{create_world_ix, delegate_account_ix, write_to_world_ix},
    pda::{find_world_pda, world_seed_hash},
    profile::{
        create_mpl_core_asset_ix, load_image_data, validate_image, ArweaveUploader, ImageSource,
        ProfilePicture,
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

    /// Creates a profile picture NFT from a local file or URL
    pub async fn create_profile_picture(
        &self,
        payer: &impl Signer,
        image_source: ImageSource,
        name: &str,
        description: Option<&str>,
        uploader: Option<ArweaveUploader>,
    ) -> Result<ProfilePicture> {
        // 1. Load and validate image
        let image_data = load_image_data(&image_source).await?;
        validate_image(&image_data)?;

        // 2. Upload image to Arweave
        let uploader = uploader.unwrap_or_default();
        let image_tx_id = uploader.upload(&image_data, Some("image/png")).await?;
        let image_uri = uploader.uri_from_tx_id(&image_tx_id);

        // 3. Create metadata JSON
        let metadata = crate::profile::Metadata::new(name, description, &image_uri);

        // 4. Upload metadata to Arweave
        let metadata_json = serde_json::to_vec(&metadata)
            .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))?;
        let metadata_tx_id = uploader
            .upload(&metadata_json, Some("application/json"))
            .await?;
        let metadata_uri = uploader.uri_from_tx_id(&metadata_tx_id);

        // 5. Create mpl-core asset
        let asset_keypair = Keypair::new();
        let asset_pubkey = asset_keypair.pubkey();

        let create_ix =
            create_mpl_core_asset_ix(&asset_pubkey, payer.pubkey(), name, &metadata_uri)?;

        // 6. Send transaction
        WorldClient::new(&self.network).send_ixs(payer, vec![create_ix], RpcLayer::BaseLayer)?;

        Ok(ProfilePicture {
            asset: asset_pubkey,
            collection: None,
            owner: payer.pubkey(),
        })
    }
}

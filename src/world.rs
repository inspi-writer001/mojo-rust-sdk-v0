use anyhow::Result;
use solana_pubkey::Pubkey;

use crate::transaction::TransactionBundle;

// Re-export build functions as associated functions on World
use crate::profile::asset::build_profile_picture_tx;
use crate::playable_characters::asset::{
    build_character_collection_tx, build_character_tx, build_select_character_tx,
};

#[cfg(feature = "native")]
use anyhow::ensure;
#[cfg(feature = "native")]
use bytemuck::{bytes_of, from_bytes, Pod, Zeroable};
#[cfg(feature = "native")]
use solana_keypair::Keypair;
#[cfg(feature = "native")]
use solana_sdk::signature::Signature;
#[cfg(feature = "native")]
use solana_signer::Signer;

#[cfg(feature = "native")]
use crate::client::{
    RpcLayer, RpcType, WorldClient, BASE_LAYER_RPC_DEVNET, BASE_LAYER_RPC_MAINNET,
    ER_LAYER_RPC_DEVNET, ER_LAYER_RPC_MAINNET,
};
#[cfg(feature = "native")]
use crate::instructions::{create_world_ix, delegate_account_ix, write_to_world_ix};
#[cfg(feature = "native")]
use crate::pda::{find_world_pda, world_seed_hash};

#[cfg(feature = "native")]
use crate::playable_characters::{
    fetch_character_data, fetch_characters_by_collection, fetch_characters_by_owner,
    fetch_collection, mint_character_ix, Character, CharacterData, CollectionData,
};

#[cfg(feature = "native")]
use crate::profile::{
    fetch_metadata_from_uri, fetch_mpl_core_asset, ProfilePictureData,
};

#[cfg(feature = "arweave")]
use crate::playable_characters::{
    create_character_ix, create_collection_ix, CharacterCollection, CharacterMetadata,
};

#[cfg(feature = "arweave")]
use crate::profile::{
    create_mpl_core_asset_ix, load_image_data, validate_image,
    ArweaveUploader, ImageSource, ProfilePicture,
};

#[cfg(feature = "native")]
pub trait MojoState: Pod + Zeroable + Copy {}

#[cfg(feature = "native")]
impl<T> MojoState for T where T: Pod + Zeroable + Copy {}

#[cfg(feature = "native")]
#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct WorldData {
    pub creator: [u8; 32],
    pub seed: [u8; 32],
    pub world_address: [u8; 32],
}

#[cfg(feature = "native")]
pub struct World {
    pub data: WorldData,
    pub network: RpcType,
}

#[cfg(not(feature = "native"))]
pub struct World;

impl World {
    // ── Layer 1: Instruction builders (always available, WASM-safe) ──

    pub fn build_profile_picture_tx(
        owner: Pubkey,
        payer: Pubkey,
        name: &str,
        metadata_uri: &str,
    ) -> Result<TransactionBundle> {
        build_profile_picture_tx(owner, payer, name, metadata_uri)
    }

    pub fn build_character_collection_tx(
        payer: Pubkey,
        name: &str,
        metadata_uri: &str,
    ) -> Result<TransactionBundle> {
        build_character_collection_tx(payer, name, metadata_uri)
    }

    pub fn build_character_tx(
        collection: &Pubkey,
        owner: Pubkey,
        payer: Pubkey,
        name: &str,
        metadata_uri: &str,
    ) -> Result<TransactionBundle> {
        build_character_tx(collection, owner, payer, name, metadata_uri)
    }

    pub fn build_select_character_tx(
        collection: &Pubkey,
        authority: Pubkey,
        buyer: Pubkey,
        payer: Pubkey,
        name: &str,
        uri: &str,
    ) -> Result<TransactionBundle> {
        build_select_character_tx(collection, authority, buyer, payer, name, uri)
    }
}

// ── Layer 2: RPC + Signing (native only) ──

#[cfg(feature = "native")]
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
            RpcType::Devnet => solana_client::rpc_client::RpcClient::new(ER_LAYER_RPC_DEVNET),
            RpcType::Mainnet => solana_client::rpc_client::RpcClient::new(ER_LAYER_RPC_MAINNET),
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

    pub fn select_character(
        &self,
        authority: &impl Signer,
        buyer: &Pubkey,
        payer: Option<&impl Signer>,
        collection: &Pubkey,
        character_name: &str,
        character_uri: &str,
    ) -> Result<Character> {
        let asset_keypair = Keypair::new();
        let asset_pubkey = asset_keypair.pubkey();

        let effective_payer = payer
            .map(|p| p.pubkey())
            .unwrap_or_else(|| authority.pubkey());
        let ix = mint_character_ix(
            &asset_pubkey,
            collection,
            authority.pubkey(),
            *buyer,
            effective_payer,
            character_name,
            character_uri,
        )?;

        let signers: Vec<&dyn Signer> =
            vec![authority as &dyn Signer, &asset_keypair as &dyn Signer];

        if let Some(p) = payer {
            WorldClient::new(&self.network).send_ixs_with_payer(
                p,
                &signers,
                vec![ix],
                RpcLayer::BaseLayer,
            )?;
        } else {
            WorldClient::new(&self.network).send_ixs_with_payer(
                authority,
                &signers,
                vec![ix],
                RpcLayer::BaseLayer,
            )?;
        }

        Ok(Character {
            asset: asset_pubkey,
            collection: *collection,
            owner: *buyer,
        })
    }

    pub async fn get_profile_picture(&self, asset: &Pubkey) -> Result<ProfilePictureData> {
        let rpc = match self.network {
            RpcType::Devnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_DEVNET),
            RpcType::Mainnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_MAINNET),
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

    pub async fn fetch_character(&self, asset: &Pubkey) -> Result<CharacterData> {
        let rpc = match self.network {
            RpcType::Devnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_DEVNET),
            RpcType::Mainnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_MAINNET),
        };

        fetch_character_data(&rpc, asset).await
    }

    pub async fn fetch_characters_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<CharacterData>> {
        let rpc = match self.network {
            RpcType::Devnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_DEVNET),
            RpcType::Mainnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_MAINNET),
        };

        let assets = fetch_characters_by_owner(&rpc, owner)?;
        let mut characters = Vec::new();
        for asset in &assets {
            let metadata_uri = &asset.base.uri;
            if let Ok(metadata) =
                crate::playable_characters::fetch_character_metadata_from_uri(metadata_uri).await
            {
                characters.push(CharacterData {
                    asset: Pubkey::default(),
                    collection: None,
                    owner: asset.base.owner,
                    name: metadata.name,
                    description: metadata.description,
                    image_uri: metadata.image,
                    metadata_uri: metadata_uri.clone(),
                });
            }
        }

        Ok(characters)
    }

    pub async fn fetch_characters_by_collection(
        &self,
        collection: &Pubkey,
    ) -> Result<Vec<CharacterData>> {
        let rpc = match self.network {
            RpcType::Devnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_DEVNET),
            RpcType::Mainnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_MAINNET),
        };

        let assets = fetch_characters_by_collection(&rpc, collection)?;
        let mut characters = Vec::new();
        for (pubkey, asset) in &assets {
            let metadata_uri = &asset.base.uri;
            if let Ok(metadata) =
                crate::playable_characters::fetch_character_metadata_from_uri(metadata_uri).await
            {
                characters.push(CharacterData {
                    asset: *pubkey,
                    collection: Some(*collection),
                    owner: asset.base.owner,
                    name: metadata.name,
                    description: metadata.description,
                    image_uri: metadata.image,
                    metadata_uri: metadata_uri.clone(),
                });
            }
        }

        Ok(characters)
    }

    pub fn fetch_collection_data(&self, collection: &Pubkey) -> Result<CollectionData> {
        let rpc = match self.network {
            RpcType::Devnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_DEVNET),
            RpcType::Mainnet => solana_client::rpc_client::RpcClient::new(BASE_LAYER_RPC_MAINNET),
        };

        fetch_collection(&rpc, collection)
    }
}

// ── Layer 3: Upload Pipeline (arweave only) ──

#[cfg(feature = "arweave")]
impl World {
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

    pub async fn create_character_collection(
        &self,
        user: &impl Signer,
        payer: Option<&impl Signer>,
        image_source: ImageSource,
        name: &str,
        description: Option<&str>,
        uploader: Option<ArweaveUploader>,
    ) -> Result<CharacterCollection> {
        let image_data = load_image_data(&image_source).await?;
        validate_image(&image_data)?;

        let uploader = uploader.unwrap_or_default();
        let image_tx_id = uploader.upload(&image_data, Some("image/png")).await?;
        let image_uri = uploader.uri_from_tx_id(&image_tx_id);

        let metadata = CharacterMetadata::new(name, description, &image_uri);

        let metadata_json = serde_json::to_vec(&metadata)
            .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))?;
        let metadata_tx_id = uploader
            .upload(&metadata_json, Some("application/json"))
            .await?;
        let metadata_uri = uploader.uri_from_tx_id(&metadata_tx_id);

        let collection_keypair = Keypair::new();
        let collection_pubkey = collection_keypair.pubkey();

        let effective_payer = payer.map(|p| p.pubkey()).unwrap_or_else(|| user.pubkey());
        let ix = create_collection_ix(&collection_pubkey, effective_payer, name, &metadata_uri)?;

        let signers: Vec<&dyn Signer> =
            vec![user as &dyn Signer, &collection_keypair as &dyn Signer];

        if let Some(p) = payer {
            WorldClient::new(&self.network).send_ixs_with_payer(
                p,
                &signers,
                vec![ix],
                RpcLayer::BaseLayer,
            )?;
        } else {
            WorldClient::new(&self.network).send_ixs_with_payer(
                user,
                &signers,
                vec![ix],
                RpcLayer::BaseLayer,
            )?;
        }

        Ok(CharacterCollection {
            collection: collection_pubkey,
            owner: user.pubkey(),
        })
    }

    pub async fn create_character(
        &self,
        user: &impl Signer,
        payer: Option<&impl Signer>,
        collection: &Pubkey,
        image_source: ImageSource,
        name: &str,
        description: Option<&str>,
        uploader: Option<ArweaveUploader>,
    ) -> Result<Character> {
        let image_data = load_image_data(&image_source).await?;
        validate_image(&image_data)?;

        let uploader = uploader.unwrap_or_default();
        let image_tx_id = uploader.upload(&image_data, Some("image/png")).await?;
        let image_uri = uploader.uri_from_tx_id(&image_tx_id);

        let metadata = CharacterMetadata::new(name, description, &image_uri);

        let metadata_json = serde_json::to_vec(&metadata)
            .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))?;
        let metadata_tx_id = uploader
            .upload(&metadata_json, Some("application/json"))
            .await?;
        let metadata_uri = uploader.uri_from_tx_id(&metadata_tx_id);

        let asset_keypair = Keypair::new();
        let asset_pubkey = asset_keypair.pubkey();

        let effective_payer = payer.map(|p| p.pubkey()).unwrap_or_else(|| user.pubkey());
        let ix = create_character_ix(
            &asset_pubkey,
            collection,
            user.pubkey(),
            effective_payer,
            name,
            &metadata_uri,
        )?;

        let signers: Vec<&dyn Signer> =
            vec![user as &dyn Signer, &asset_keypair as &dyn Signer];

        if let Some(p) = payer {
            WorldClient::new(&self.network).send_ixs_with_payer(
                p,
                &signers,
                vec![ix],
                RpcLayer::BaseLayer,
            )?;
        } else {
            WorldClient::new(&self.network).send_ixs_with_payer(
                user,
                &signers,
                vec![ix],
                RpcLayer::BaseLayer,
            )?;
        }

        Ok(Character {
            asset: asset_pubkey,
            collection: *collection,
            owner: user.pubkey(),
        })
    }
}

use anyhow::Result;
use mpl_core::instructions::{CreateCollectionV2Builder, CreateV2Builder};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use crate::transaction::TransactionBundle;

pub fn create_collection_ix(
    collection: &Pubkey,
    payer: Pubkey,
    name: &str,
    uri: &str,
) -> Result<Instruction> {
    let mut builder = CreateCollectionV2Builder::new();
    builder
        .collection(*collection)
        .payer(payer)
        .name(name.to_string())
        .uri(uri.to_string());

    let ix = builder.instruction();
    Ok(ix)
}

pub fn create_character_ix(
    asset: &Pubkey,
    collection: &Pubkey,
    owner: Pubkey,
    payer: Pubkey,
    name: &str,
    uri: &str,
) -> Result<Instruction> {
    let mut builder = CreateV2Builder::new();
    builder
        .asset(*asset)
        .collection(Some(*collection))
        .authority(Some(owner))
        .owner(Some(owner))
        .payer(payer)
        .name(name.to_string())
        .uri(uri.to_string());

    let ix = builder.instruction();
    Ok(ix)
}

pub fn mint_character_ix(
    asset: &Pubkey,
    collection: &Pubkey,
    authority: Pubkey,
    owner: Pubkey,
    payer: Pubkey,
    name: &str,
    uri: &str,
) -> Result<Instruction> {
    let mut builder = CreateV2Builder::new();
    builder
        .asset(*asset)
        .collection(Some(*collection))
        .authority(Some(authority))
        .owner(Some(owner))
        .payer(payer)
        .name(name.to_string())
        .uri(uri.to_string());

    let ix = builder.instruction();
    Ok(ix)
}

pub fn build_character_collection_tx(
    payer: Pubkey,
    name: &str,
    metadata_uri: &str,
) -> Result<TransactionBundle> {
    let collection_keypair = Keypair::new();
    let collection_pubkey = collection_keypair.pubkey();

    let ix = create_collection_ix(&collection_pubkey, payer, name, metadata_uri)?;

    Ok(TransactionBundle {
        instructions: vec![ix],
        signers: vec![collection_keypair],
    })
}

pub fn build_character_tx(
    collection: &Pubkey,
    owner: Pubkey,
    payer: Pubkey,
    name: &str,
    metadata_uri: &str,
) -> Result<TransactionBundle> {
    let asset_keypair = Keypair::new();
    let asset_pubkey = asset_keypair.pubkey();

    let ix = create_character_ix(&asset_pubkey, collection, owner, payer, name, metadata_uri)?;

    Ok(TransactionBundle {
        instructions: vec![ix],
        signers: vec![asset_keypair],
    })
}

pub fn build_select_character_tx(
    collection: &Pubkey,
    authority: Pubkey,
    buyer: Pubkey,
    payer: Pubkey,
    name: &str,
    uri: &str,
) -> Result<TransactionBundle> {
    let asset_keypair = Keypair::new();
    let asset_pubkey = asset_keypair.pubkey();

    let ix = mint_character_ix(&asset_pubkey, collection, authority, buyer, payer, name, uri)?;

    Ok(TransactionBundle {
        instructions: vec![ix],
        signers: vec![asset_keypair],
    })
}

#[cfg(feature = "native")]
pub fn fetch_character(
    rpc: &solana_client::rpc_client::RpcClient,
    asset: &Pubkey,
) -> Result<mpl_core::Asset> {
    use crate::error::WorldError;

    let account_data = rpc
        .get_account_data(asset)
        .map_err(|e| WorldError::AccountNotFound(format!("Failed to fetch account: {}", e)))?;

    if account_data.is_empty() {
        return Err(
            WorldError::AccountNotFound(format!("Account {} does not exist", asset)).into(),
        );
    }

    let asset = mpl_core::Asset::from_bytes(&account_data)
        .map_err(|e| WorldError::AssetDeserializationError(format!("{}", e)))?;
    Ok(*asset)
}

#[cfg(feature = "native")]
pub fn fetch_collection(
    rpc: &solana_client::rpc_client::RpcClient,
    collection: &Pubkey,
) -> Result<crate::playable_characters::types::CollectionData> {
    use crate::error::WorldError;
    use mpl_core::Collection;

    let account_data = rpc.get_account_data(collection).map_err(|e| {
        WorldError::AccountNotFound(format!("Failed to fetch collection: {}", e))
    })?;

    if account_data.is_empty() {
        return Err(WorldError::AccountNotFound(format!(
            "Collection {} does not exist",
            collection
        ))
        .into());
    }

    let col = Collection::from_bytes(&account_data)
        .map_err(|e| WorldError::AssetDeserializationError(format!("{}", e)))?;

    Ok(crate::playable_characters::types::CollectionData {
        collection: *collection,
        update_authority: col.base.update_authority,
        name: col.base.name,
        uri: col.base.uri,
        num_minted: col.base.num_minted,
        current_size: col.base.current_size,
    })
}

#[cfg(feature = "native")]
pub async fn fetch_character_data(
    rpc: &solana_client::rpc_client::RpcClient,
    asset_pubkey: &Pubkey,
) -> Result<crate::playable_characters::types::CharacterData> {
    let mpl_asset = fetch_character(rpc, asset_pubkey)?;

    let owner = mpl_asset.base.owner;
    let collection = None;
    let metadata_uri = mpl_asset.base.uri;

    let metadata = fetch_character_metadata_from_uri(&metadata_uri).await?;

    Ok(crate::playable_characters::types::CharacterData {
        asset: *asset_pubkey,
        collection,
        owner,
        name: metadata.name,
        description: metadata.description,
        image_uri: metadata.image,
        metadata_uri,
    })
}

#[cfg(feature = "native")]
pub async fn fetch_character_metadata_from_uri(metadata_uri: &str) -> Result<crate::playable_characters::types::CharacterMetadata> {
    use anyhow::Context;
    use crate::error::WorldError;

    let response = reqwest::get(metadata_uri).await.map_err(|e| {
        WorldError::MetadataFetchError(format!("Failed to download metadata: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(
            WorldError::MetadataFetchError(format!("HTTP error: {}", response.status())).into(),
        );
    }

    let metadata: crate::playable_characters::types::CharacterMetadata = response
        .json()
        .await
        .context("Failed to parse character metadata JSON")?;

    Ok(metadata)
}

#[cfg(feature = "native")]
pub fn fetch_characters_by_owner(
    rpc: &solana_client::rpc_client::RpcClient,
    owner: &Pubkey,
) -> Result<Vec<mpl_core::Asset>> {
    use crate::error::WorldError;
    use solana_client::rpc_filter::{Memcmp, RpcFilterType};

    let mpl_core_id = mpl_core::ID;

    let owner_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(1, owner.to_bytes().to_vec()));
    let key_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, vec![1]));

    let config = solana_client::rpc_config::RpcProgramAccountsConfig {
        filters: Some(vec![key_filter, owner_filter]),
        account_config: Default::default(),
        ..Default::default()
    };

    let accounts = rpc
        .get_program_accounts_with_config(&mpl_core_id, config)
        .map_err(|e| WorldError::AccountNotFound(format!("Failed to fetch accounts: {}", e)))?;

    let mut assets = Vec::new();
    for (_pubkey, account) in &accounts {
        if let Ok(asset) = mpl_core::Asset::from_bytes(&account.data) {
            assets.push(*asset);
        }
    }

    Ok(assets)
}

#[cfg(feature = "native")]
pub fn fetch_characters_by_collection(
    rpc: &solana_client::rpc_client::RpcClient,
    collection: &Pubkey,
) -> Result<Vec<(Pubkey, mpl_core::Asset)>> {
    use crate::error::WorldError;
    use solana_client::rpc_filter::{Memcmp, RpcFilterType};

    let mpl_core_id = mpl_core::ID;

    let key_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, vec![1]));
    let collection_variant_filter =
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(33, vec![2]));
    let collection_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        34,
        collection.to_bytes().to_vec(),
    ));

    let config = solana_client::rpc_config::RpcProgramAccountsConfig {
        filters: Some(vec![key_filter, collection_variant_filter, collection_filter]),
        account_config: Default::default(),
        ..Default::default()
    };

    let accounts = rpc
        .get_program_accounts_with_config(&mpl_core_id, config)
        .map_err(|e| WorldError::AccountNotFound(format!("Failed to fetch accounts: {}", e)))?;

    let mut assets = Vec::new();
    for (pubkey, account) in &accounts {
        if let Ok(asset) = mpl_core::Asset::from_bytes(&account.data) {
            assets.push((*pubkey, *asset));
        }
    }

    Ok(assets)
}

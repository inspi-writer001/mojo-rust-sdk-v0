use anyhow::{Context, Result};
use mpl_core::instructions::{CreateCollectionV2Builder, CreateV2Builder};
use mpl_core::{Asset, Collection};
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::WorldError;
use crate::playable_characters::types::{CharacterData, CharacterMetadata, CollectionData};

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

pub fn fetch_character(rpc: &RpcClient, asset: &Pubkey) -> Result<Asset> {
    let account_data = rpc
        .get_account_data(asset)
        .map_err(|e| WorldError::AccountNotFound(format!("Failed to fetch account: {}", e)))?;

    if account_data.is_empty() {
        return Err(
            WorldError::AccountNotFound(format!("Account {} does not exist", asset)).into(),
        );
    }

    let asset = Asset::from_bytes(&account_data)
        .map_err(|e| WorldError::AssetDeserializationError(format!("{}", e)))?;
    Ok(*asset)
}

pub fn fetch_collection(rpc: &RpcClient, collection: &Pubkey) -> Result<CollectionData> {
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

    Ok(CollectionData {
        collection: *collection,
        update_authority: col.base.update_authority,
        name: col.base.name,
        uri: col.base.uri,
        num_minted: col.base.num_minted,
        current_size: col.base.current_size,
    })
}

pub async fn fetch_character_data(rpc: &RpcClient, asset_pubkey: &Pubkey) -> Result<CharacterData> {
    let mpl_asset = fetch_character(rpc, asset_pubkey)?;

    let owner = mpl_asset.base.owner;
    let collection = None;
    let metadata_uri = mpl_asset.base.uri;

    let metadata = fetch_character_metadata_from_uri(&metadata_uri).await?;

    Ok(CharacterData {
        asset: *asset_pubkey,
        collection,
        owner,
        name: metadata.name,
        description: metadata.description,
        image_uri: metadata.image,
        metadata_uri,
    })
}

pub async fn fetch_character_metadata_from_uri(metadata_uri: &str) -> Result<CharacterMetadata> {
    let response = reqwest::get(metadata_uri).await.map_err(|e| {
        WorldError::MetadataFetchError(format!("Failed to download metadata: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(
            WorldError::MetadataFetchError(format!("HTTP error: {}", response.status())).into(),
        );
    }

    let metadata: CharacterMetadata = response
        .json()
        .await
        .context("Failed to parse character metadata JSON")?;

    Ok(metadata)
}

pub fn fetch_characters_by_owner(
    rpc: &RpcClient,
    owner: &Pubkey,
) -> Result<Vec<Asset>> {
    use solana_client::rpc_filter::{Memcmp, RpcFilterType};

    let mpl_core_id = mpl_core::ID;

    // MPL Core asset account layout: byte 0 is Key discriminator (Uninitialized=0, AssetV1=1),
    // then owner pubkey starts at offset 1
    let owner_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(1, owner.to_bytes().to_vec()));
    // Filter for AssetV1 key discriminator
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
        if let Ok(asset) = Asset::from_bytes(&account.data) {
            assets.push(*asset);
        }
    }

    Ok(assets)
}

pub fn fetch_characters_by_collection(
    rpc: &RpcClient,
    collection: &Pubkey,
) -> Result<Vec<(Pubkey, Asset)>> {
    use solana_client::rpc_filter::{Memcmp, RpcFilterType};

    let mpl_core_id = mpl_core::ID;

    // For assets with a collection, the layout has the UpdateAuthority enum variant
    // at offset 33 (after key byte + owner 32 bytes). The collection variant is 2,
    // followed by the collection pubkey at offset 34.
    let key_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, vec![1])); // AssetV1
    let collection_variant_filter =
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(33, vec![2])); // UpdateAuthority::Collection
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
        if let Ok(asset) = Asset::from_bytes(&account.data) {
            assets.push((*pubkey, *asset));
        }
    }

    Ok(assets)
}

use anyhow::{Context, Result};
use mpl_core::instructions::{CreateV1Builder, UpdateV1Builder};
use mpl_core::Asset;
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::WorldError;
use crate::profile::types::Metadata;

pub fn create_mpl_core_asset_ix(
    asset: &Pubkey,
    owner: Pubkey,
    payer: Pubkey,
    name: &str,
    uri: &str,
) -> Result<Instruction> {
    let mut builder = CreateV1Builder::new();
    builder
        .asset(*asset)
        .owner(Some(owner))
        .payer(payer)
        .name(name.to_string())
        .uri(uri.to_string());

    let create_ix = builder.instruction();

    Ok(create_ix)
}

pub fn update_mpl_core_asset_ix(
    asset: &Pubkey,
    authority: Pubkey,
    payer: Pubkey,
    new_name: Option<String>,
    new_uri: Option<String>,
) -> Result<Instruction> {
    let mut builder = UpdateV1Builder::new();
    builder
        .asset(*asset)
        .payer(payer)
        .authority(Some(authority));

    if let Some(name) = new_name {
        builder.new_name(name);
    }

    if let Some(uri) = new_uri {
        builder.new_uri(uri);
    }

    let ix = builder.instruction();
    Ok(ix)
}

pub fn fetch_mpl_core_asset(rpc: &RpcClient, asset: &Pubkey) -> Result<Asset> {
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

pub async fn fetch_metadata_from_uri(metadata_uri: &str) -> Result<Metadata> {
    let response = reqwest::get(metadata_uri).await.map_err(|e| {
        WorldError::MetadataFetchError(format!("Failed to download metadata: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(
            WorldError::MetadataFetchError(format!("HTTP error: {}", response.status())).into(),
        );
    }

    let metadata: Metadata = response
        .json()
        .await
        .context("Failed to parse metadata JSON")?;

    Ok(metadata)
}

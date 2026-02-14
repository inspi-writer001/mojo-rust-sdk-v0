use anyhow::Result;
use mpl_core::instructions::CreateV1Builder;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use crate::transaction::TransactionBundle;

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

pub fn build_profile_picture_tx(
    owner: Pubkey,
    payer: Pubkey,
    name: &str,
    metadata_uri: &str,
) -> Result<TransactionBundle> {
    let asset_keypair = Keypair::new();
    let asset_pubkey = asset_keypair.pubkey();

    let ix = create_mpl_core_asset_ix(&asset_pubkey, owner, payer, name, metadata_uri)?;

    Ok(TransactionBundle {
        instructions: vec![ix],
        signers: vec![asset_keypair],
    })
}

#[cfg(feature = "native")]
pub fn fetch_mpl_core_asset(
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
pub async fn fetch_metadata_from_uri(metadata_uri: &str) -> Result<crate::profile::types::Metadata> {
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

    let metadata: crate::profile::types::Metadata = response
        .json()
        .await
        .context("Failed to parse metadata JSON")?;

    Ok(metadata)
}

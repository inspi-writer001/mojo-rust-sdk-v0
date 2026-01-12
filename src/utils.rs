use anyhow::Result;
use solana_instruction::Instruction;
use solana_sdk::signature::Signature;
use solana_signer::Signer;

use crate::{
    client::{RpcLayer, RpcType, WorldClient},
    error::WorldError,
    profile::{load_image_data, validate_image, ArweaveUploader, ImageSource},
};

/// Upload image + metadata to Arweave, returning the metadata URI.
pub async fn upload_metadata<T: serde::Serialize>(
    image_source: ImageSource,
    build_metadata: impl FnOnce(String) -> T,
    uploader: Option<ArweaveUploader>,
) -> Result<String, WorldError> {
    let image_data = load_image_data(&image_source).await?;
    validate_image(&image_data)?;

    let uploader = uploader.unwrap_or_default();
    let image_tx_id = uploader
        .upload(&image_data, Some("image/png"))
        .await
        .map_err(|e| WorldError::MetadataUploadError(e.to_string()))?;
    let image_uri = uploader.uri_from_tx_id(&image_tx_id);

    let metadata = build_metadata(image_uri);

    let metadata_json = serde_json::to_vec(&metadata)
        .map_err(|e| WorldError::MetadataUploadError(format!("serialize metadata: {}", e)))?;
    let metadata_tx_id = uploader
        .upload(&metadata_json, Some("application/json"))
        .await
        .map_err(|e| WorldError::MetadataUploadError(e.to_string()))?;
    Ok(uploader.uri_from_tx_id(&metadata_tx_id))
}

/// Send a single instruction with the provided payer/signers on the chosen layer.
pub fn send_with_signers(
    network: RpcType,
    payer: &dyn Signer,
    signers: &[&dyn Signer],
    ix: Instruction,
    layer: RpcLayer,
) -> Result<Signature, WorldError> {
    let filtered: Vec<&dyn Signer> = signers
        .iter()
        .copied()
        .filter(|s| s.pubkey() != payer.pubkey())
        .collect();
    WorldClient::new(&network).send_ixs_with_payer(payer, &filtered, vec![ix], layer)
        .map_err(|e| WorldError::RpcError(e.to_string()))
}

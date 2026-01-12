use anyhow::Result;
use mpl_core::instructions::{
    CreateCollectionV1Builder, CreateV1Builder, TransferV1Builder, UpdateV1Builder,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

pub fn create_mpl_core_badge_collection_ix(
    collection: &Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    name: &str,
    uri: &str,
) -> Result<Instruction> {
    let mut builder = CreateCollectionV1Builder::new();
    builder
        .collection(*collection)
        .payer(payer)
        .update_authority(Some(update_authority))
        .name(name.to_string())
        .uri(uri.to_string());

    Ok(builder.instruction())
}

pub fn create_mpl_core_badge_ix(
    asset: &Pubkey,
    collection: Pubkey,
    authority: Pubkey,
    owner: Pubkey,
    payer: Pubkey,
    name: &str,
    uri: &str,
) -> Result<Instruction> {
    let mut builder = CreateV1Builder::new();
    builder
        .asset(*asset)
        .collection(Some(collection))
        .authority(Some(authority))
        .payer(payer)
        .owner(Some(owner))
        .update_authority(Some(authority))
        .name(name.to_string())
        .uri(uri.to_string());

    Ok(builder.instruction())
}

pub fn update_mpl_core_badge_ix(
    asset: &Pubkey,
    collection: Option<Pubkey>,
    payer: Pubkey,
    authority: Pubkey,
    new_name: Option<&str>,
    new_uri: Option<&str>,
) -> Instruction {
    let mut builder = UpdateV1Builder::new();
    builder
        .asset(*asset)
        .collection(collection)
        .payer(payer)
        .authority(Some(authority));

    if let Some(name) = new_name {
        builder.new_name(name.to_string());
    }
    if let Some(uri) = new_uri {
        builder.new_uri(uri.to_string());
    }

    builder.instruction()
}

pub fn transfer_mpl_core_badge_ix(
    asset: &Pubkey,
    collection: Option<Pubkey>,
    payer: Pubkey,
    authority: Pubkey,
    new_owner: Pubkey,
) -> Instruction {
    let mut builder = TransferV1Builder::new();
    builder
        .asset(*asset)
        .collection(collection)
        .payer(payer)
        .authority(Some(authority))
        .new_owner(new_owner);

    builder.instruction()
}


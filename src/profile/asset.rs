use anyhow::Result;
use mpl_core::instructions::CreateV1Builder;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

/// Create an mpl-core CreateV1 instruction for profile picture NFT
pub fn create_mpl_core_asset_ix(
    asset: &Pubkey,
    payer: Pubkey,
    name: &str,
    uri: &str,
) -> Result<Instruction> {
    let mut builder = CreateV1Builder::new();
    builder
        .asset(*asset)
        .payer(payer)
        .name(name.to_string())
        .uri(uri.to_string());

    let create_ix = builder.instruction();

    Ok(create_ix)
}

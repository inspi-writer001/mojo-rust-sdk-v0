use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_sdk::signature::Signature;
use solana_signer::Signer;
use solana_transaction::{self, Transaction};

pub const BASE_LAYER_RPC_DEVNET: &str = "https://api.devnet.solana.com";
pub const ER_LAYER_RPC_DEVNET: &str = "https://devnet-eu.magicblock.app";

pub const BASE_LAYER_RPC_MAINNET: &str = "https://api.mainnet-beta.solana.com";
pub const ER_LAYER_RPC_MAINNET: &str = "https://mainnet-beta-eu.magicblock.app";

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum RpcType {
    Mainnet,
    Devnet,
}
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum RpcLayer {
    BaseLayer,
    Ephemeral,
}

pub struct WorldClient {
    pub cluster: RpcType,
    // pub rpc: Option<RpcClient>,
}

impl WorldClient {
    pub fn new(rpc_type: &RpcType) -> Self {
        Self { cluster: *rpc_type }
    }

    pub fn send_ixs(
        &mut self,
        payer: &impl Signer,
        instructions: Vec<Instruction>,
        layer: RpcLayer,
    ) -> Result<Signature> {
        let rpc = match self.cluster {
            RpcType::Devnet => match layer {
                RpcLayer::BaseLayer => RpcClient::new(BASE_LAYER_RPC_DEVNET),
                RpcLayer::Ephemeral => RpcClient::new(ER_LAYER_RPC_DEVNET),
            },
            RpcType::Mainnet => match layer {
                RpcLayer::BaseLayer => RpcClient::new(BASE_LAYER_RPC_MAINNET),
                RpcLayer::Ephemeral => RpcClient::new(ER_LAYER_RPC_MAINNET),
            },
        };

        let blockhash = rpc.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        );
        let tx = rpc.send_and_confirm_transaction(&tx)?;
        Ok(tx)
    }
}

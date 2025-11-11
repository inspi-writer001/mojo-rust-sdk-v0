use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_signer::Signer;
use solana_transaction::{self, Transaction};

pub struct WorldClient {
    pub rpc: RpcClient,
}

impl WorldClient {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url),
        }
    }

    pub fn send_ixs(&self, payer: &impl Signer, instructions: Vec<Instruction>) -> Result<()> {
        let blockhash = self.rpc.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        );
        self.rpc.send_and_confirm_transaction(&tx)?;
        Ok(())
    }
}

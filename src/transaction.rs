use solana_instruction::Instruction;
use solana_keypair::Keypair;

pub struct TransactionBundle {
    pub instructions: Vec<Instruction>,
    pub signers: Vec<Keypair>,
}

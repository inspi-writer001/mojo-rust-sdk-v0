use anyhow::{ensure, Result};
use bytemuck::{bytes_of, from_bytes, Pod, Zeroable};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use crate::{
    client::WorldClient,
    instructions::{create_world_ix, delegate_account_ix, write_to_world_ix},
    pda::{find_world_pda, world_seed_hash},
};

pub trait MojoState: Pod + Zeroable + Copy {}

impl<T> MojoState for T where T: Pod + Zeroable + Copy {}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct World {
    pub creator: [u8; 32],
    pub seed: [u8; 32],
}

impl World {
    pub fn create_world(client: &WorldClient, payer: &impl Signer, name: &str) -> Result<Pubkey> {
        let (world_pda, _) = find_world_pda(&payer.pubkey(), name);
        let seed_hash = world_seed_hash(&payer.pubkey(), name);
        let ix = create_world_ix(
            payer.pubkey(),
            world_pda,
            seed_hash,
            bytes_of(&World {
                creator: payer.pubkey().to_bytes(),
                seed: seed_hash,
            }),
        );

        client.send_ixs(payer, vec![ix])?;
        Ok(world_pda)
    }

    pub fn create_state<T: MojoState>(
        client: &WorldClient,
        payer: &impl Signer,
        name: &str,
        initial_state: &T,
    ) -> Result<Pubkey> {
        let (state_pda, _) = find_world_pda(&payer.pubkey(), name);
        let seed_hash = world_seed_hash(&payer.pubkey(), name);
        let ix = create_world_ix(
            payer.pubkey(),
            state_pda,
            seed_hash,
            bytes_of(initial_state),
        );

        let delegate_ix = delegate_account_ix(
            payer.pubkey(),
            state_pda,
            seed_hash,
            bytes_of(initial_state),
        );

        client.send_ixs(payer, vec![ix, delegate_ix])?;
        Ok(state_pda)
    }

    pub fn write_state<T: MojoState>(
        client: &WorldClient,
        payer: &impl Signer,
        name: &str,
        new_state: &T,
    ) -> Result<()> {
        let (world_pda, _) = find_world_pda(&payer.pubkey(), name);
        let seed_hash = world_seed_hash(&payer.pubkey(), name);
        let ix = write_to_world_ix(payer.pubkey(), world_pda, seed_hash, bytes_of(new_state));

        client.send_ixs(payer, vec![ix])?;
        Ok(())
    }

    pub fn read_state<T: MojoState>(client: &WorldClient, owner: &Pubkey, name: &str) -> Result<T> {
        let (world_pda, _) = find_world_pda(owner, name);
        let data = client.rpc.get_account_data(&world_pda)?;
        let required_len = core::mem::size_of::<T>();
        ensure!(
            data.len() >= required_len,
            "account data length {} smaller than expected {}",
            data.len(),
            required_len
        );

        let state = *from_bytes::<T>(&data[..required_len]);
        Ok(state)
    }
}

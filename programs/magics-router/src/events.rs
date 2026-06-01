use anchor_lang::prelude::*;

#[event]
pub struct Deposited {
    pub agent_id: [u8; 32],
    pub mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Withdrawn {
    pub agent_id: [u8; 32],
    pub mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Cast {
    pub agent_id: [u8; 32],
    pub seal: [u8; 32],
    pub strategy: Pubkey,
    pub nonce: u64,
    pub data_hash: [u8; 32],
}

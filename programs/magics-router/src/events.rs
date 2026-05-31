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

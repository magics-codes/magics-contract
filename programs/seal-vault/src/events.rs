use anchor_lang::prelude::*;

#[event]
pub struct SealMinted {
    pub owner: Pubkey,
    pub seal_id: [u8; 32],
    pub signer: Pubkey,
    pub expiry: i64,
}

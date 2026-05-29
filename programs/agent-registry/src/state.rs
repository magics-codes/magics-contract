use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;

use crate::constants::AGENT_TAG;

/// Lifecycle of an agent. Mirrors the EVM `Status` enum one-for-one.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Active,
    Paused,
    Halted,
}

/// An agent record: an owner, a strategy, and a seal bound under a stable id.
/// The registry never moves funds and never runs strategy code — it just gives
/// a name to the triple and lets the owner change the binding within the rules.
#[account]
#[derive(InitSpace)]
pub struct Agent {
    pub id: [u8; 32],
    pub owner: Pubkey,
    pub strategy: Pubkey,
    pub seal: [u8; 32],
    pub created_at: i64,
    pub updated_at: i64,
    pub status: Status,
    #[max_len(64)]
    pub name: String,
    pub bump: u8,
}

/// Per-owner monotonic counter. Bumped on every summon so two agents over the
/// same (owner, strategy, seal) still land on distinct ids — the Solana stand-in
/// for the EVM registry's `_counter[msg.sender]`.
#[account]
#[derive(InitSpace)]
pub struct OwnerCounter {
    pub owner: Pubkey,
    pub count: u64,
    pub bump: u8,
}

/// Deterministic agent id. Mirrors the EVM
/// `keccak256(chainId, owner, strategy, sealId, ++counter)`; the program id of
/// the registry stands in for `chainId` via the domain tag.
pub fn derive_agent_id(
    owner: &Pubkey,
    strategy: &Pubkey,
    seal_id: &[u8; 32],
    counter: u64,
) -> [u8; 32] {
    keccak::hashv(&[
        AGENT_TAG,
        owner.as_ref(),
        strategy.as_ref(),
        seal_id,
        &counter.to_le_bytes(),
    ])
    .0
}

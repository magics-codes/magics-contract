use anchor_lang::prelude::*;
use magics_common::Seal;

/// Singleton that binds the vault to a single router program. The EVM vault
/// took the router address in its constructor and made it `immutable`; here the
/// admin sets it once at `initialize`, and only that router's cast-authority
/// PDA may advance seal state.
#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub router_program: Pubkey,
    pub bump: u8,
}

/// One seal: the boundary parameters paired with the mutable usage counters the
/// vault advances on every consumed cast. Keyed by a PDA at
/// `[SEAL_SEED, seal_id]`, where `seal_id` is the boundary-field hash.
#[account]
#[derive(InitSpace)]
pub struct SealRecord {
    pub seal: Seal,
    pub owner: Pubkey,
    pub seal_id: [u8; 32],
    /// Next valid nonce.
    pub nonce: u64,
    /// Start of the current 24h window, unix seconds.
    pub window_start: i64,
    /// Token value spent inside the current window.
    pub window_spent: u64,
    pub revoked: bool,
    #[max_len(64)]
    pub revoke_reason: String,
    pub bump: u8,
}

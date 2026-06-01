use anchor_lang::prelude::*;

/// One agent's credited balance of one mint. The set of these PDAs is the Solana
/// shape of the EVM router's `_balances[agentId][token]` mapping. Funds live in a
/// shared vault token account; this record is the ledger entry that says how much
/// of it the agent may move.
#[account]
#[derive(InitSpace)]
pub struct Balance {
    pub agent_id: [u8; 32],
    pub mint: Pubkey,
    pub amount: u64,
    pub bump: u8,
}

/// The transient cast context — the Solana stand-in for the EVM router's
/// transient-storage slot. Created by `cast` after the seal verifies, read by
/// the strategy's `pull` / `push`, and closed when the cast returns, so the
/// budget API only opens inside a live, seal-verified cast.
///
/// Zero-copy on purpose: the strategy CPI happens mid-instruction, and a regular
/// account wouldn't flush our writes until the handler exits — too late for the
/// nested call to read. `load_init` writes straight to the account data.
#[account(zero_copy)]
#[repr(C)]
pub struct ActiveCast {
    pub agent_id: [u8; 32],
    pub strategy: Pubkey,
    pub strategy_authority: Pubkey,
    pub bump: u8,
    pub _padding: [u8; 7],
}

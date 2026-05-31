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

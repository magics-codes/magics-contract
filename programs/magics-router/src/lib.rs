//! # magics-router
//!
//! The hinge. Every cast goes through here. Holds the per-agent token ledger,
//! validates the seal before delegating to a strategy, and opens a transient
//! budget the strategy can spend from.
//!
//! The active-cast context lives in a short-lived PDA that survives the
//! strategy CPI and is closed at the end of the cast — the Solana stand-in for
//! the EVM router's transient-storage slot. Strategies call back into pull /
//! push proving identity with their own program PDA, so they can't lie about
//! whose budget they're touching.

use anchor_lang::prelude::*;

pub mod constants;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("2c2FBbgCpB2VPhqMDsTX6VXJEhcnczSSTw5eR3DZrpUu");

#[program]
pub mod magics_router {
    use super::*;

    /// Credit an agent's balance from the owner's tokens.
    pub fn deposit(ctx: Context<Deposit>, agent_id: [u8; 32], amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, agent_id, amount)
    }

    /// Move tokens out of an agent's balance to a destination.
    pub fn withdraw(ctx: Context<Withdraw>, agent_id: [u8; 32], amount: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, agent_id, amount)
    }

    /// The cast entry point. Verify the seal, then hand control to the strategy
    /// under a transient budget.
    pub fn cast<'info>(
        ctx: Context<'_, '_, '_, 'info, CastAction<'info>>,
        agent_id: [u8; 32],
        deadline: i64,
        data: Vec<u8>,
    ) -> Result<()> {
        instructions::cast::handler(ctx, agent_id, deadline, data)
    }
}

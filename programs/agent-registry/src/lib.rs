//! # agent-registry
//!
//! The book of names. An agent record pairs an owner, a strategy, and a seal —
//! and gives the triple a stable id you can refer to from anywhere else in the
//! system. The registry never holds funds and never executes anything; all it
//! does is bind names to tuples and let owners change the binding within the
//! rules.

use anchor_lang::prelude::*;

pub mod constants;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("31SxC6ivUkHdcUnvR23wqGyJgdmiHWR7UWZuWW42cYCR");

#[program]
pub mod agent_registry {
    use super::*;

    /// Bind (owner, strategy, seal) into a fresh agent.
    pub fn summon(
        ctx: Context<Summon>,
        agent_id: [u8; 32],
        seal_id: [u8; 32],
        strategy: Pubkey,
        name: String,
    ) -> Result<()> {
        instructions::summon::handler(ctx, agent_id, seal_id, strategy, name)
    }

    /// Pause an agent.
    pub fn pause(ctx: Context<UpdateAgent>, agent_id: [u8; 32], reason: String) -> Result<()> {
        instructions::status::pause(ctx, agent_id, reason)
    }

    /// Resume a paused agent.
    pub fn resume(ctx: Context<UpdateAgent>, agent_id: [u8; 32]) -> Result<()> {
        instructions::status::resume(ctx, agent_id)
    }

    /// Halt an agent permanently.
    pub fn halt(ctx: Context<UpdateAgent>, agent_id: [u8; 32], reason: String) -> Result<()> {
        instructions::status::halt(ctx, agent_id, reason)
    }

    /// Rotate the seal bound to an agent.
    pub fn rotate_seal(
        ctx: Context<RotateSeal>,
        agent_id: [u8; 32],
        new_seal_id: [u8; 32],
    ) -> Result<()> {
        instructions::rotate_seal::handler(ctx, agent_id, new_seal_id)
    }
}

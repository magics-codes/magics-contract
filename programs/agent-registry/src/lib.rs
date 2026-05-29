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

declare_id!("9qEVeDELh9wSRfKFcWohWkYjW2MfoKXepFTXeqN9TFsN");

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
}

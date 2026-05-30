use anchor_lang::prelude::*;
use magics_common::MagicsError;

use crate::constants::AGENT_SEED;
use crate::events::AgentStatusChanged;
use crate::state::{Agent, Status};

const MAX_REASON_LEN: usize = 64;

#[derive(Accounts)]
#[instruction(agent_id: [u8; 32])]
pub struct UpdateAgent<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [AGENT_SEED, agent_id.as_ref()],
        bump = agent.bump,
        has_one = owner @ MagicsError::NotOwner,
    )]
    pub agent: Account<'info, Agent>,
}

/// Pause an agent. A halted agent is terminal and cannot be paused.
pub fn pause(ctx: Context<UpdateAgent>, _agent_id: [u8; 32], reason: String) -> Result<()> {
    require!(reason.len() <= MAX_REASON_LEN, MagicsError::ReasonTooLong);
    let agent = &mut ctx.accounts.agent;
    let prev = agent.status;
    require!(prev != Status::Halted, MagicsError::AgentNotActive);
    transition(agent, prev, Status::Paused, reason)
}

/// Resume a paused agent. A halted agent stays halted.
pub fn resume(ctx: Context<UpdateAgent>, _agent_id: [u8; 32]) -> Result<()> {
    let agent = &mut ctx.accounts.agent;
    let prev = agent.status;
    require!(prev != Status::Halted, MagicsError::AgentNotActive);
    transition(agent, prev, Status::Active, String::new())
}

/// Halt an agent for good. Reachable from any state; there is no way back.
pub fn halt(ctx: Context<UpdateAgent>, _agent_id: [u8; 32], reason: String) -> Result<()> {
    require!(reason.len() <= MAX_REASON_LEN, MagicsError::ReasonTooLong);
    let agent = &mut ctx.accounts.agent;
    let prev = agent.status;
    transition(agent, prev, Status::Halted, reason)
}

fn transition(agent: &mut Agent, prev: Status, to: Status, reason: String) -> Result<()> {
    agent.status = to;
    agent.updated_at = Clock::get()?.unix_timestamp;
    emit!(AgentStatusChanged {
        agent_id: agent.id,
        from: prev,
        to,
        reason,
    });
    Ok(())
}

use anchor_lang::prelude::*;
use magics_common::MagicsError;
use seal_vault::state::SealRecord;

use crate::constants::AGENT_SEED;
use crate::events::AgentSealRotated;
use crate::state::Agent;

#[derive(Accounts)]
#[instruction(agent_id: [u8; 32], new_seal_id: [u8; 32])]
pub struct RotateSeal<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [AGENT_SEED, agent_id.as_ref()],
        bump = agent.bump,
        has_one = owner @ MagicsError::NotOwner,
    )]
    pub agent: Account<'info, Agent>,

    /// The replacement seal — validated under the vault and confirmed to belong
    /// to the caller, same check `summon` makes.
    #[account(
        seeds = [b"seal", new_seal_id.as_ref()],
        bump = new_seal_record.bump,
        seeds::program = seal_vault::ID,
        has_one = owner @ MagicsError::NotOwner,
    )]
    pub new_seal_record: Account<'info, SealRecord>,
}

/// Point an agent at a different seal without re-summoning. Lets an owner rotate
/// the session key under a live agent — handy after a `revoke`.
pub fn handler(ctx: Context<RotateSeal>, _agent_id: [u8; 32], new_seal_id: [u8; 32]) -> Result<()> {
    let agent = &mut ctx.accounts.agent;
    let old_seal = agent.seal;
    agent.seal = new_seal_id;
    agent.updated_at = Clock::get()?.unix_timestamp;

    emit!(AgentSealRotated {
        agent_id: agent.id,
        old_seal,
        new_seal: new_seal_id,
    });
    Ok(())
}

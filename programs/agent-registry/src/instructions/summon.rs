use anchor_lang::prelude::*;
use magics_common::MagicsError;
use seal_vault::state::SealRecord;

use crate::constants::{AGENT_SEED, COUNTER_SEED, MAX_NAME_LENGTH};
use crate::events::AgentSummoned;
use crate::state::{derive_agent_id, Agent, OwnerCounter, Status};

#[derive(Accounts)]
#[instruction(agent_id: [u8; 32], seal_id: [u8; 32], strategy: Pubkey)]
pub struct Summon<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + OwnerCounter::INIT_SPACE,
        seeds = [COUNTER_SEED, owner.key().as_ref()],
        bump
    )]
    pub counter: Account<'info, OwnerCounter>,

    /// The seal being bound. Validated as the real PDA under the seal vault and
    /// confirmed to belong to the caller — the Solana read of `vault.ownerOf`.
    #[account(
        seeds = [b"seal", seal_id.as_ref()],
        bump = seal_record.bump,
        seeds::program = seal_vault::ID,
        has_one = owner @ MagicsError::NotOwner,
    )]
    pub seal_record: Account<'info, SealRecord>,

    #[account(
        init,
        payer = owner,
        space = 8 + Agent::INIT_SPACE,
        seeds = [AGENT_SEED, agent_id.as_ref()],
        bump
    )]
    pub agent: Account<'info, Agent>,

    pub system_program: Program<'info, System>,
}

/// Bind (owner, strategy, seal) into a fresh agent. `agent_id` is the caller's
/// claim of the derived id; we bump the owner's counter, recompute, and reject a
/// mismatch — so the agent's PDA always matches its contents.
pub fn handler(
    ctx: Context<Summon>,
    agent_id: [u8; 32],
    seal_id: [u8; 32],
    strategy: Pubkey,
    name: String,
) -> Result<()> {
    require!(!name.is_empty(), MagicsError::AgentNameEmpty);
    require!(name.len() <= MAX_NAME_LENGTH, MagicsError::AgentNameTooLong);
    require!(strategy != Pubkey::default(), MagicsError::ZeroAddress);

    let owner = ctx.accounts.owner.key();

    let counter = &mut ctx.accounts.counter;
    if counter.owner == Pubkey::default() {
        counter.owner = owner;
        counter.bump = ctx.bumps.counter;
    }
    counter.count = counter
        .count
        .checked_add(1)
        .ok_or(MagicsError::NumericalOverflow)?;

    let expected = derive_agent_id(&owner, &strategy, &seal_id, counter.count);
    require!(expected == agent_id, MagicsError::AgentIdMismatch);

    let now = Clock::get()?.unix_timestamp;
    let agent = &mut ctx.accounts.agent;
    agent.id = agent_id;
    agent.owner = owner;
    agent.strategy = strategy;
    agent.seal = seal_id;
    agent.created_at = now;
    agent.updated_at = now;
    agent.status = Status::Active;
    agent.name = name.clone();
    agent.bump = ctx.bumps.agent;

    emit!(AgentSummoned {
        owner,
        agent_id,
        strategy,
        seal: seal_id,
        name,
    });
    Ok(())
}

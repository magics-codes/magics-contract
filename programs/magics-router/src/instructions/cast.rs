use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::{get_return_data, invoke};
use anchor_lang::solana_program::hash::hash;
use agent_registry::state::{Agent, Status};
use magics_common::{data_hash, MagicsError, CAST_AUTHORITY_SEED};

use crate::constants::{ACTIVE_SEED, STRATEGY_AUTH_SEED};
use crate::events::Cast;
use crate::state::ActiveCast;

/// Arguments handed to a strategy's `execute`. The wire shape of the strategy
/// interface — the Solana analog of `IStrategy.execute(agentId, owner, data)`.
#[derive(AnchorSerialize)]
struct ExecuteArgs {
    agent_id: [u8; 32],
    owner: Pubkey,
    data: Vec<u8>,
}

#[derive(Accounts)]
#[instruction(agent_id: [u8; 32])]
pub struct CastAction<'info> {
    #[account(mut)]
    pub caster: Signer<'info>,

    #[account(
        seeds = [b"agent", agent_id.as_ref()],
        bump = agent.bump,
        seeds::program = agent_registry::ID,
    )]
    pub agent: Account<'info, Agent>,

    /// The seal backing this agent. Mutated by the vault's `verify_and_consume`
    /// CPI (nonce + window); the router never writes it directly.
    #[account(
        mut,
        seeds = [b"seal", agent.seal.as_ref()],
        bump = seal_record.bump,
        seeds::program = seal_vault::ID,
    )]
    pub seal_record: Account<'info, seal_vault::state::SealRecord>,

    #[account(
        seeds = [b"config"],
        bump = seal_vault_config.bump,
        seeds::program = seal_vault::ID,
    )]
    pub seal_vault_config: Account<'info, seal_vault::state::Config>,

    /// CHECK: router cast-authority PDA; signs the verify CPI on the router's behalf.
    #[account(seeds = [CAST_AUTHORITY_SEED], bump)]
    pub cast_authority: UncheckedAccount<'info>,

    /// CHECK: instructions sysvar, forwarded to the vault for the ed25519 check.
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,

    #[account(
        init,
        payer = caster,
        space = 8 + std::mem::size_of::<ActiveCast>(),
        seeds = [ACTIVE_SEED, agent_id.as_ref()],
        bump
    )]
    pub active_cast: AccountLoader<'info, ActiveCast>,

    /// CHECK: the strategy program to hand control to; must equal agent.strategy.
    pub strategy_program: UncheckedAccount<'info>,

    /// CHECK: the seal vault program, address-validated, for the verify CPI.
    #[account(address = seal_vault::ID)]
    pub seal_vault_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// The single entry point through which an agent acts. Verifies the seal, opens
/// a transient budget, hands control to the strategy, then closes the budget.
/// Any accounts the strategy needs (its own state, the router accounts it pulls
/// and pushes against) come in as `remaining_accounts` and are forwarded.
pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, CastAction<'info>>,
    agent_id: [u8; 32],
    deadline: i64,
    data: Vec<u8>,
) -> Result<()> {
    {
        let agent = &ctx.accounts.agent;
        require!(agent.status == Status::Active, MagicsError::AgentNotActive);
        require_keys_eq!(
            ctx.accounts.strategy_program.key(),
            agent.strategy,
            MagicsError::NotStrategy
        );
    }
    let now = Clock::get()?.unix_timestamp;
    require!(deadline >= now, MagicsError::CastDeadlinePassed);

    let seal_id = ctx.accounts.agent.seal;
    let owner = ctx.accounts.agent.owner;
    let strategy = ctx.accounts.agent.strategy;
    let dh = data_hash(&data);

    // 1) Verify + consume the seal, signed by the router's cast authority.
    let ca_bump = ctx.bumps.cast_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[CAST_AUTHORITY_SEED, &[ca_bump]]];
    let cpi_accounts = seal_vault::cpi::accounts::VerifyAndConsume {
        config: ctx.accounts.seal_vault_config.to_account_info(),
        record: ctx.accounts.seal_record.to_account_info(),
        router_authority: ctx.accounts.cast_authority.to_account_info(),
        instructions_sysvar: ctx.accounts.instructions_sysvar.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.seal_vault_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    seal_vault::cpi::verify_and_consume(cpi_ctx, seal_id, agent_id, deadline, dh, 0)?;

    // The vault returns the consumed nonce in its return data.
    let nonce = match get_return_data() {
        Some((program, bytes)) if program == seal_vault::ID && bytes.len() >= 8 => {
            u64::from_le_bytes(bytes[..8].try_into().unwrap())
        }
        _ => 0,
    };

    // 2) Open the transient budget context for the strategy to spend from.
    let (strategy_authority, _) = Pubkey::find_program_address(&[STRATEGY_AUTH_SEED], &strategy);
    {
        let mut active = ctx.accounts.active_cast.load_init()?;
        active.agent_id = agent_id;
        active.strategy = strategy;
        active.strategy_authority = strategy_authority;
        active.bump = ctx.bumps.active_cast;
    }

    // 3) Hand control to the strategy. Built as a raw instruction so any
    //    conforming strategy program can sit here — the pluggable seam.
    let discriminator = hash(b"global:execute").to_bytes();
    let mut ix_data = Vec::with_capacity(8 + 64 + data.len());
    ix_data.extend_from_slice(&discriminator[..8]);
    ExecuteArgs {
        agent_id,
        owner,
        data,
    }
    .serialize(&mut ix_data)?;

    let metas: Vec<AccountMeta> = ctx
        .remaining_accounts
        .iter()
        .map(|a| AccountMeta {
            pubkey: *a.key,
            is_signer: a.is_signer,
            is_writable: a.is_writable,
        })
        .collect();
    let ix = Instruction {
        program_id: strategy,
        accounts: metas,
        data: ix_data,
    };
    let mut infos = ctx.remaining_accounts.to_vec();
    infos.push(ctx.accounts.strategy_program.to_account_info());
    invoke(&ix, &infos)?;

    // 4) Close the transient context — refund rent to the caster. Outside a
    //    cast there is no active context, so the budget API stays shut.
    {
        let active_info = ctx.accounts.active_cast.to_account_info();
        let dest = ctx.accounts.caster.to_account_info();
        let refunded = active_info.lamports();
        **active_info.try_borrow_mut_lamports()? = 0;
        **dest.try_borrow_mut_lamports()? = dest
            .lamports()
            .checked_add(refunded)
            .ok_or(MagicsError::NumericalOverflow)?;
    }

    emit!(Cast {
        agent_id,
        seal: seal_id,
        strategy,
        nonce,
        data_hash: dh,
    });
    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use agent_registry::state::Agent;
use magics_common::MagicsError;

use crate::constants::{BALANCE_SEED, VAULT_AUTH_SEED, VAULT_TOKEN_SEED};
use crate::events::Deposited;
use crate::state::Balance;

#[derive(Accounts)]
#[instruction(agent_id: [u8; 32])]
pub struct Deposit<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// The agent receiving the credit. Validated under the registry and owned by
    /// the caller — the program wiring is fixed at compile time, so the registry
    /// id is a constant, not stored config.
    #[account(
        seeds = [b"agent", agent_id.as_ref()],
        bump = agent.bump,
        seeds::program = agent_registry::ID,
        has_one = owner @ MagicsError::NotOwner,
    )]
    pub agent: Account<'info, Agent>,

    pub mint: Account<'info, Mint>,

    #[account(mut, token::mint = mint, token::authority = owner)]
    pub owner_token: Account<'info, TokenAccount>,

    /// Shared custody for this mint, owned by the vault authority PDA.
    #[account(
        init_if_needed,
        payer = owner,
        seeds = [VAULT_TOKEN_SEED, mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault_authority,
    )]
    pub vault_token: Account<'info, TokenAccount>,

    /// CHECK: PDA authority over every vault token account; never signs here.
    #[account(seeds = [VAULT_AUTH_SEED], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + Balance::INIT_SPACE,
        seeds = [BALANCE_SEED, agent_id.as_ref(), mint.key().as_ref()],
        bump
    )]
    pub balance: Account<'info, Balance>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// Move tokens from the owner into the agent's credited balance.
pub fn handler(ctx: Context<Deposit>, agent_id: [u8; 32], amount: u64) -> Result<()> {
    require!(amount > 0, MagicsError::AmountZero);

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.owner_token.to_account_info(),
                to: ctx.accounts.vault_token.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
    )?;

    let balance = &mut ctx.accounts.balance;
    if balance.mint == Pubkey::default() {
        balance.agent_id = agent_id;
        balance.mint = ctx.accounts.mint.key();
        balance.bump = ctx.bumps.balance;
    }
    balance.amount = balance
        .amount
        .checked_add(amount)
        .ok_or(MagicsError::NumericalOverflow)?;

    emit!(Deposited {
        agent_id,
        mint: ctx.accounts.mint.key(),
        amount,
    });
    Ok(())
}

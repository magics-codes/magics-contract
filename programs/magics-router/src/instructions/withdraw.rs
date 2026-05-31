use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use agent_registry::state::Agent;
use magics_common::MagicsError;

use crate::constants::{BALANCE_SEED, VAULT_AUTH_SEED, VAULT_TOKEN_SEED};
use crate::events::Withdrawn;
use crate::state::Balance;

#[derive(Accounts)]
#[instruction(agent_id: [u8; 32])]
pub struct Withdraw<'info> {
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"agent", agent_id.as_ref()],
        bump = agent.bump,
        seeds::program = agent_registry::ID,
        has_one = owner @ MagicsError::NotOwner,
    )]
    pub agent: Account<'info, Agent>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_TOKEN_SEED, mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault_authority,
    )]
    pub vault_token: Account<'info, TokenAccount>,

    /// CHECK: PDA authority over the vault token account; signs the transfer out.
    #[account(seeds = [VAULT_AUTH_SEED], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut, token::mint = mint)]
    pub destination: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [BALANCE_SEED, agent_id.as_ref(), mint.key().as_ref()],
        bump = balance.bump,
    )]
    pub balance: Account<'info, Balance>,

    pub token_program: Program<'info, Token>,
}

/// Move tokens out of the agent's credited balance to a destination account.
pub fn handler(ctx: Context<Withdraw>, _agent_id: [u8; 32], amount: u64) -> Result<()> {
    require!(amount > 0, MagicsError::AmountZero);

    let balance = &mut ctx.accounts.balance;
    require!(balance.amount >= amount, MagicsError::InsufficientBalance);
    balance.amount -= amount;

    let bump = ctx.bumps.vault_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[VAULT_AUTH_SEED, &[bump]]];
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    emit!(Withdrawn {
        agent_id: balance.agent_id,
        mint: ctx.accounts.mint.key(),
        amount,
    });
    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use magics_common::MagicsError;

use crate::constants::{ACTIVE_SEED, BALANCE_SEED, VAULT_AUTH_SEED, VAULT_TOKEN_SEED};
use crate::state::{ActiveCast, Balance};

/// Shared account set for the strategy-facing budget calls. Both `pull` and
/// `push` are gated the same way: an open active-cast context for this agent,
/// plus the strategy's own PDA signing — proof the call is happening inside a
/// live cast and is coming from the agent's registered strategy.
#[derive(Accounts)]
#[instruction(agent_id: [u8; 32])]
pub struct Budget<'info> {
    /// The active strategy's `[STRATEGY_AUTH_SEED]` PDA, signing.
    pub strategy_authority: Signer<'info>,

    #[account(
        seeds = [ACTIVE_SEED, agent_id.as_ref()],
        bump,
    )]
    pub active_cast: AccountLoader<'info, ActiveCast>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [BALANCE_SEED, agent_id.as_ref(), mint.key().as_ref()],
        bump = balance.bump,
    )]
    pub balance: Account<'info, Balance>,

    #[account(
        mut,
        seeds = [VAULT_TOKEN_SEED, mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault_authority,
    )]
    pub vault_token: Account<'info, TokenAccount>,

    /// CHECK: PDA authority over the vault token account.
    #[account(seeds = [VAULT_AUTH_SEED], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// The strategy's own token account — pull credits it, push drains it.
    #[account(mut, token::mint = mint, token::authority = strategy_authority)]
    pub strategy_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Budget<'info> {
    /// Confirm the signer is the strategy named in the open cast for this agent.
    fn gate(&self, agent_id: &[u8; 32]) -> Result<()> {
        let active = self.active_cast.load()?;
        require!(&active.agent_id == agent_id, MagicsError::CastContextMismatch);
        require_keys_eq!(
            self.strategy_authority.key(),
            active.strategy_authority,
            MagicsError::NotStrategy
        );
        Ok(())
    }
}

/// Draw tokens from the agent's balance into the strategy's account.
pub fn pull(ctx: Context<Budget>, agent_id: [u8; 32], amount: u64) -> Result<()> {
    ctx.accounts.gate(&agent_id)?;
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
                to: ctx.accounts.strategy_token.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;
    Ok(())
}

/// Return tokens from the strategy back into the agent's balance.
pub fn push(ctx: Context<Budget>, agent_id: [u8; 32], amount: u64) -> Result<()> {
    ctx.accounts.gate(&agent_id)?;
    require!(amount > 0, MagicsError::AmountZero);

    // The strategy authority signed this instruction, so its signature carries
    // into the nested transfer — no extra signer seeds needed.
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.strategy_token.to_account_info(),
                to: ctx.accounts.vault_token.to_account_info(),
                authority: ctx.accounts.strategy_authority.to_account_info(),
            },
        ),
        amount,
    )?;

    let balance = &mut ctx.accounts.balance;
    balance.amount = balance
        .amount
        .checked_add(amount)
        .ok_or(MagicsError::NumericalOverflow)?;
    Ok(())
}

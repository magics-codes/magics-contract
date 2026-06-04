//! # passive-yield
//!
//! The reference strategy — the Solana twin of `PassiveYieldStrategy.sol`. Two
//! ops, two leaves:
//!   - `0x01` compound: pull idle asset from the router and deposit it into a
//!     yield vault, banking the shares against the agent.
//!   - `0x02` harvest: redeem shares and push the proceeds back into the agent's
//!     router balance.
//!
//! `execute` is the strategy seam the router calls into. It never trusts a
//! caller: every move goes through the router's budget API, which only opens
//! inside a live, seal-verified cast. The strategy signs those budget calls —
//! and the yield-vault calls — with its own `[b"strategy"]` PDA, the identity
//! the router checks against the open cast.

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("HPEdHcgxJVQX2VRcYM2fMXn963sXXmwhuxC9SwWLQD6n");

/// Must match the router's `STRATEGY_AUTH_SEED`.
const STRATEGY_SEED: &[u8] = b"strategy";
const POSITION_SEED: &[u8] = b"position";

const OP_COMPOUND: u8 = 0x01;
const OP_HARVEST: u8 = 0x02;

#[program]
pub mod passive_yield {
    use super::*;

    /// One cast. `data` is `[op: u8]` optionally followed by `[amount: u64 LE]`;
    /// a missing or zero amount means "all available".
    pub fn execute(
        mut ctx: Context<Execute>,
        agent_id: [u8; 32],
        _owner: Pubkey,
        data: Vec<u8>,
    ) -> Result<()> {
        require!(!data.is_empty(), StrategyError::UnknownOp);
        let op = data[0];
        let amount = if data.len() >= 9 {
            u64::from_le_bytes(data[1..9].try_into().unwrap())
        } else {
            0
        };

        let bump = ctx.bumps.strategy_authority;
        let signer: &[&[&[u8]]] = &[&[STRATEGY_SEED, &[bump]]];

        // Seed the position record on first touch.
        if ctx.accounts.position.agent_id == [0u8; 32] {
            ctx.accounts.position.agent_id = agent_id;
            ctx.accounts.position.bump = ctx.bumps.position;
        }

        match op {
            OP_COMPOUND => compound(&mut ctx, agent_id, amount, signer),
            OP_HARVEST => harvest(&mut ctx, agent_id, amount, signer),
            _ => err!(StrategyError::UnknownOp),
        }
    }
}

fn compound(
    ctx: &mut Context<Execute>,
    agent_id: [u8; 32],
    requested: u64,
    signer: &[&[&[u8]]],
) -> Result<()> {
    let available = ctx.accounts.router_balance.amount;
    let spent = if requested == 0 { available } else { requested };
    require!(
        spent > 0 && spent <= available,
        StrategyError::NothingToCompound
    );

    // Draw the idle asset out of the agent's router balance.
    magics_router::cpi::pull(
        CpiContext::new_with_signer(
            ctx.accounts.router_program.to_account_info(),
            magics_router::cpi::accounts::Budget {
                strategy_authority: ctx.accounts.strategy_authority.to_account_info(),
                active_cast: ctx.accounts.router_active_cast.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info(),
                balance: ctx.accounts.router_balance.to_account_info(),
                vault_token: ctx.accounts.router_vault_token.to_account_info(),
                vault_authority: ctx.accounts.router_vault_authority.to_account_info(),
                strategy_token: ctx.accounts.strategy_asset_token.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
            signer,
        ),
        agent_id,
        spent,
    )?;

    // Deposit it into the yield vault; shares are minted 1:1.
    mock_yield::cpi::deposit(
        CpiContext::new_with_signer(
            ctx.accounts.mock_program.to_account_info(),
            mock_yield::cpi::accounts::Deposit {
                depositor: ctx.accounts.strategy_authority.to_account_info(),
                vault: ctx.accounts.mock_vault.to_account_info(),
                asset_mint: ctx.accounts.asset_mint.to_account_info(),
                share_mint: ctx.accounts.share_mint.to_account_info(),
                depositor_asset: ctx.accounts.strategy_asset_token.to_account_info(),
                underlying_vault: ctx.accounts.mock_underlying.to_account_info(),
                depositor_share: ctx.accounts.strategy_share_token.to_account_info(),
                vault_authority: ctx.accounts.mock_vault_authority.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
            signer,
        ),
        spent,
    )?;

    let position = &mut ctx.accounts.position;
    position.shares = position
        .shares
        .checked_add(spent)
        .ok_or(StrategyError::Overflow)?;
    Ok(())
}

fn harvest(
    ctx: &mut Context<Execute>,
    agent_id: [u8; 32],
    requested: u64,
    signer: &[&[&[u8]]],
) -> Result<()> {
    let have = ctx.accounts.position.shares;
    let burn = if requested == 0 { have } else { requested };
    require!(burn > 0 && burn <= have, StrategyError::NothingToHarvest);

    // Redeem shares back into the strategy's asset account.
    mock_yield::cpi::redeem(
        CpiContext::new_with_signer(
            ctx.accounts.mock_program.to_account_info(),
            mock_yield::cpi::accounts::Redeem {
                redeemer: ctx.accounts.strategy_authority.to_account_info(),
                vault: ctx.accounts.mock_vault.to_account_info(),
                asset_mint: ctx.accounts.asset_mint.to_account_info(),
                share_mint: ctx.accounts.share_mint.to_account_info(),
                redeemer_share: ctx.accounts.strategy_share_token.to_account_info(),
                underlying_vault: ctx.accounts.mock_underlying.to_account_info(),
                redeemer_asset: ctx.accounts.strategy_asset_token.to_account_info(),
                vault_authority: ctx.accounts.mock_vault_authority.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
            signer,
        ),
        burn,
    )?;

    // Push everything that came back into the agent's router balance.
    ctx.accounts.strategy_asset_token.reload()?;
    let received = ctx.accounts.strategy_asset_token.amount;
    magics_router::cpi::push(
        CpiContext::new_with_signer(
            ctx.accounts.router_program.to_account_info(),
            magics_router::cpi::accounts::Budget {
                strategy_authority: ctx.accounts.strategy_authority.to_account_info(),
                active_cast: ctx.accounts.router_active_cast.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info(),
                balance: ctx.accounts.router_balance.to_account_info(),
                vault_token: ctx.accounts.router_vault_token.to_account_info(),
                vault_authority: ctx.accounts.router_vault_authority.to_account_info(),
                strategy_token: ctx.accounts.strategy_asset_token.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
            signer,
        ),
        agent_id,
        received,
    )?;

    let position = &mut ctx.accounts.position;
    position.shares -= burn;
    Ok(())
}

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub agent_id: [u8; 32],
    pub shares: u64,
    pub bump: u8,
}

#[derive(Accounts)]
#[instruction(agent_id: [u8; 32])]
pub struct Execute<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + Position::INIT_SPACE,
        seeds = [POSITION_SEED, agent_id.as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    /// CHECK: the strategy's signing identity; the router checks this against
    /// the open cast. Seeds prove we own it.
    #[account(seeds = [STRATEGY_SEED], bump)]
    pub strategy_authority: UncheckedAccount<'info>,

    pub asset_mint: Account<'info, Mint>,
    /// CHECK: the yield vault's share mint; validated inside mock-yield.
    #[account(mut)]
    pub share_mint: UncheckedAccount<'info>,

    #[account(mut, token::mint = asset_mint, token::authority = strategy_authority)]
    pub strategy_asset_token: Account<'info, TokenAccount>,
    /// CHECK: strategy's share account; validated inside mock-yield.
    #[account(mut)]
    pub strategy_share_token: UncheckedAccount<'info>,

    // ── Router side (validated inside the router) ──────────────────────────
    /// CHECK: address-checked router program.
    #[account(address = magics_router::ID)]
    pub router_program: UncheckedAccount<'info>,
    /// CHECK: open cast context; the router validates it.
    pub router_active_cast: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"balance", agent_id.as_ref(), asset_mint.key().as_ref()],
        bump = router_balance.bump,
        seeds::program = magics_router::ID,
    )]
    pub router_balance: Account<'info, magics_router::state::Balance>,
    /// CHECK: router vault token account.
    #[account(mut)]
    pub router_vault_token: UncheckedAccount<'info>,
    /// CHECK: router vault authority.
    pub router_vault_authority: UncheckedAccount<'info>,

    // ── Yield vault side (validated inside mock-yield) ─────────────────────
    /// CHECK: address-checked yield-vault program.
    #[account(address = mock_yield::ID)]
    pub mock_program: UncheckedAccount<'info>,
    /// CHECK: yield vault record.
    pub mock_vault: UncheckedAccount<'info>,
    /// CHECK: yield vault underlying account.
    #[account(mut)]
    pub mock_underlying: UncheckedAccount<'info>,
    /// CHECK: yield vault authority.
    pub mock_vault_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum StrategyError {
    #[msg("unknown strategy op")]
    UnknownOp,
    #[msg("nothing to compound")]
    NothingToCompound,
    #[msg("nothing to harvest")]
    NothingToHarvest,
    #[msg("numerical overflow")]
    Overflow,
}

//! # mock-yield
//!
//! A toy ERC-4626-style vault, the Solana twin of `MockYieldVault.sol`. Shares
//! are minted 1:1 with deposited assets; on redeem they pay out
//! `shares * (10000 + yield_bps) / 10000`. The yield is a knob the tests turn,
//! funded by topping up the vault's underlying — there is nothing clever here,
//! it just lets a strategy round-trip through a yield source.

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("4339iYHiD6y52hcCWrUdN7tjH3rSEwnmyKdQZfFpngWC");

const VAULT_SEED: &[u8] = b"yvault";
const AUTH_SEED: &[u8] = b"yauth";
const SHARE_SEED: &[u8] = b"yshare";
const UNDER_SEED: &[u8] = b"yunder";
const BPS: u128 = 10_000;

#[program]
pub mod mock_yield {
    use super::*;

    /// Stand up a vault for one asset mint: its share mint and underlying account.
    pub fn init_vault(ctx: Context<InitVault>, yield_bps: u16) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.asset_mint = ctx.accounts.asset_mint.key();
        vault.share_mint = ctx.accounts.share_mint.key();
        vault.yield_bps = yield_bps;
        vault.bump = ctx.bumps.vault;
        vault.auth_bump = ctx.bumps.vault_authority;
        Ok(())
    }

    /// Turn the yield knob. Applied at redeem time.
    pub fn set_yield_bps(ctx: Context<SetYield>, yield_bps: u16) -> Result<()> {
        ctx.accounts.vault.yield_bps = yield_bps;
        Ok(())
    }

    /// Deposit assets, mint shares 1:1.
    pub fn deposit(ctx: Context<Deposit>, assets: u64) -> Result<()> {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.depositor_asset.to_account_info(),
                    to: ctx.accounts.underlying_vault.to_account_info(),
                    authority: ctx.accounts.depositor.to_account_info(),
                },
            ),
            assets,
        )?;

        let asset_mint = ctx.accounts.asset_mint.key();
        let bump = ctx.accounts.vault.auth_bump;
        let signer: &[&[&[u8]]] = &[&[AUTH_SEED, asset_mint.as_ref(), &[bump]]];
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.share_mint.to_account_info(),
                    to: ctx.accounts.depositor_share.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer,
            ),
            assets,
        )?;
        Ok(())
    }

    /// Burn shares, pay out assets plus the configured yield.
    pub fn redeem(ctx: Context<Redeem>, shares: u64) -> Result<()> {
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.share_mint.to_account_info(),
                    from: ctx.accounts.redeemer_share.to_account_info(),
                    authority: ctx.accounts.redeemer.to_account_info(),
                },
            ),
            shares,
        )?;

        let yield_bps = ctx.accounts.vault.yield_bps as u128;
        let assets = (shares as u128 * (BPS + yield_bps) / BPS) as u64;

        let asset_mint = ctx.accounts.asset_mint.key();
        let bump = ctx.accounts.vault.auth_bump;
        let signer: &[&[&[u8]]] = &[&[AUTH_SEED, asset_mint.as_ref(), &[bump]]];
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.underlying_vault.to_account_info(),
                    to: ctx.accounts.redeemer_asset.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer,
            ),
            assets,
        )?;
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub asset_mint: Pubkey,
    pub share_mint: Pubkey,
    pub yield_bps: u16,
    pub bump: u8,
    pub auth_bump: u8,
}

#[derive(Accounts)]
pub struct InitVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub asset_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + Vault::INIT_SPACE,
        seeds = [VAULT_SEED, asset_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: PDA authority over the share mint and underlying account.
    #[account(seeds = [AUTH_SEED, asset_mint.key().as_ref()], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [SHARE_SEED, asset_mint.key().as_ref()],
        bump,
        mint::decimals = asset_mint.decimals,
        mint::authority = vault_authority,
    )]
    pub share_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        seeds = [UNDER_SEED, asset_mint.key().as_ref()],
        bump,
        token::mint = asset_mint,
        token::authority = vault_authority,
    )]
    pub underlying_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetYield<'info> {
    #[account(mut, seeds = [VAULT_SEED, vault.asset_mint.as_ref()], bump = vault.bump)]
    pub vault: Account<'info, Vault>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub depositor: Signer<'info>,

    #[account(
        seeds = [VAULT_SEED, asset_mint.key().as_ref()],
        bump = vault.bump,
        has_one = asset_mint,
        has_one = share_mint,
    )]
    pub vault: Account<'info, Vault>,

    pub asset_mint: Account<'info, Mint>,
    #[account(mut)]
    pub share_mint: Account<'info, Mint>,

    #[account(mut, token::mint = asset_mint, token::authority = depositor)]
    pub depositor_asset: Account<'info, TokenAccount>,

    #[account(mut, seeds = [UNDER_SEED, asset_mint.key().as_ref()], bump)]
    pub underlying_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = share_mint, token::authority = depositor)]
    pub depositor_share: Account<'info, TokenAccount>,

    /// CHECK: PDA authority, signs the share mint.
    #[account(seeds = [AUTH_SEED, asset_mint.key().as_ref()], bump = vault.auth_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Redeem<'info> {
    pub redeemer: Signer<'info>,

    #[account(
        seeds = [VAULT_SEED, asset_mint.key().as_ref()],
        bump = vault.bump,
        has_one = asset_mint,
        has_one = share_mint,
    )]
    pub vault: Account<'info, Vault>,

    pub asset_mint: Account<'info, Mint>,
    #[account(mut)]
    pub share_mint: Account<'info, Mint>,

    #[account(mut, token::mint = share_mint, token::authority = redeemer)]
    pub redeemer_share: Account<'info, TokenAccount>,

    #[account(mut, seeds = [UNDER_SEED, asset_mint.key().as_ref()], bump)]
    pub underlying_vault: Account<'info, TokenAccount>,

    #[account(mut, token::mint = asset_mint, token::authority = redeemer)]
    pub redeemer_asset: Account<'info, TokenAccount>,

    /// CHECK: PDA authority, signs the underlying transfer out.
    #[account(seeds = [AUTH_SEED, asset_mint.key().as_ref()], bump = vault.auth_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

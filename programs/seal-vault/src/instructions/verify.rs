use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::set_return_data;
use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;
use magics_common::{cast_message, verify_cast_signature, CAST_AUTHORITY_SEED, MagicsError};

use crate::constants::{CONFIG_SEED, SEAL_SEED, WINDOW};
use crate::events::SealConsumed;
use crate::state::{Config, SealRecord};

#[derive(Accounts)]
#[instruction(seal_id: [u8; 32])]
pub struct VerifyAndConsume<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [SEAL_SEED, seal_id.as_ref()],
        bump = record.bump,
    )]
    pub record: Account<'info, SealRecord>,

    /// The router's cast-authority PDA, signing on the router's behalf. Checked
    /// against the router this vault was bound to — the gate that says "only the
    /// router may advance seal state."
    pub router_authority: Signer<'info>,

    /// CHECK: instructions sysvar, address-validated; read to confirm the
    /// Ed25519 verify instruction the session key signed.
    #[account(address = INSTRUCTIONS_ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,
}

/// Verify a cast against the seal and consume one nonce. CPI-only: the caller
/// must be the bound router's cast authority. Returns the consumed nonce in the
/// instruction return data so the router can log it.
pub fn handler(
    ctx: Context<VerifyAndConsume>,
    _seal_id: [u8; 32],
    agent_id: [u8; 32],
    deadline: i64,
    data_hash: [u8; 32],
    call_value: u64,
) -> Result<()> {
    // Caller must be the cast authority of the router we were bound to.
    let (expected_authority, _) =
        Pubkey::find_program_address(&[CAST_AUTHORITY_SEED], &ctx.accounts.config.router_program);
    require_keys_eq!(
        ctx.accounts.router_authority.key(),
        expected_authority,
        MagicsError::NotRouter
    );

    let now = Clock::get()?.unix_timestamp;
    let record = &mut ctx.accounts.record;
    require!(!record.revoked, MagicsError::SealRevoked);

    let seal = record.seal;
    require!(now < seal.expiry, MagicsError::SealExpired);
    require!(deadline >= now, MagicsError::CastDeadlinePassed);
    require!(call_value <= seal.value_cap, MagicsError::SealCapBreached);

    // Roll the daily window if it has elapsed, then enforce the daily cap.
    let mut window_start = record.window_start;
    let mut window_spent = record.window_spent;
    if now >= window_start + WINDOW {
        window_start = now;
        window_spent = 0;
    }
    let projected = window_spent
        .checked_add(call_value)
        .ok_or(MagicsError::NumericalOverflow)?;
    require!(projected <= seal.daily_cap, MagicsError::SealCapBreached);

    // The session key must have signed exactly this cast. The Ed25519 native
    // program already proved the signature; we just confirm it covered our
    // (signer, message) pair.
    let message = cast_message(&crate::ID, &agent_id, record.nonce, deadline, &data_hash);
    verify_cast_signature(
        &ctx.accounts.instructions_sysvar.to_account_info(),
        &seal.signer,
        &message,
    )?;

    let nonce = record.nonce;
    record.nonce = nonce.checked_add(1).ok_or(MagicsError::NumericalOverflow)?;
    record.window_start = window_start;
    record.window_spent = projected;

    emit!(SealConsumed {
        seal_id: record.seal_id,
        value: call_value,
        nonce,
    });

    // Hand the consumed nonce back to the router via return data.
    set_return_data(&nonce.to_le_bytes());
    Ok(())
}

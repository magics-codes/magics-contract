use anchor_lang::prelude::*;
use magics_common::MagicsError;

use crate::constants::{MAX_REASON_LEN, SEAL_SEED};
use crate::events::SealRevoked;
use crate::state::SealRecord;

#[derive(Accounts)]
#[instruction(seal_id: [u8; 32])]
pub struct Revoke<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SEAL_SEED, seal_id.as_ref()],
        bump = record.bump,
        has_one = owner @ MagicsError::NotOwner,
    )]
    pub record: Account<'info, SealRecord>,
}

/// Revoke a single seal. Only the owner may call; double-revocation reverts.
pub fn handler(ctx: Context<Revoke>, _seal_id: [u8; 32], reason: String) -> Result<()> {
    require!(reason.len() <= MAX_REASON_LEN, MagicsError::ReasonTooLong);
    let record = &mut ctx.accounts.record;
    require!(!record.revoked, MagicsError::SealRevoked);

    record.revoked = true;
    record.revoke_reason = reason.clone();

    emit!(SealRevoked {
        owner: record.owner,
        seal_id: record.seal_id,
        reason,
    });
    Ok(())
}

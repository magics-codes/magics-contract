use anchor_lang::prelude::*;
use magics_common::{MagicsError, Seal};

use crate::constants::SEAL_SEED;
use crate::events::SealMinted;
use crate::state::SealRecord;

/// Boundary parameters the caller supplies. `created_at` is deliberately absent
/// — the vault stamps it from the chain clock so it can't be back-dated.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SealArgs {
    pub signer: Pubkey,
    pub target: Pubkey,
    pub selector: [u8; 8],
    pub value_cap: u64,
    pub daily_cap: u64,
    pub expiry: i64,
    pub scope_hash: [u8; 32],
}

#[derive(Accounts)]
#[instruction(seal_id: [u8; 32])]
pub struct MintSeal<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + SealRecord::INIT_SPACE,
        seeds = [SEAL_SEED, seal_id.as_ref()],
        bump
    )]
    pub record: Account<'info, SealRecord>,

    pub system_program: Program<'info, System>,
}

/// Mint (or re-issue) a seal under the caller. `seal_id` is the client's claim
/// of the boundary-field hash; we recompute it and reject any mismatch, so the
/// PDA address always corresponds to the parameters stored inside it.
pub fn handler(ctx: Context<MintSeal>, seal_id: [u8; 32], args: SealArgs) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    require!(args.expiry > now, MagicsError::SealExpired);
    require!(
        args.signer != Pubkey::default() && args.target != Pubkey::default(),
        MagicsError::ZeroAddress
    );

    let owner = ctx.accounts.owner.key();
    let seal = Seal {
        signer: args.signer,
        target: args.target,
        selector: args.selector,
        value_cap: args.value_cap,
        daily_cap: args.daily_cap,
        expiry: args.expiry,
        created_at: now,
        scope_hash: args.scope_hash,
    };
    require!(seal.id(&owner) == seal_id, MagicsError::SealIdMismatch);

    let record = &mut ctx.accounts.record;

    // Re-issue path: an existing record must belong to the caller and already
    // be revoked. A live record can't be silently overwritten.
    if record.owner != Pubkey::default() {
        require_keys_eq!(record.owner, owner, MagicsError::NotOwner);
        require!(record.revoked, MagicsError::AlreadyInitialised);
    }

    record.seal = seal;
    record.owner = owner;
    record.seal_id = seal_id;
    record.nonce = 0;
    record.window_start = now;
    record.window_spent = 0;
    record.revoked = false;
    record.revoke_reason = String::new();
    record.bump = ctx.bumps.record;

    emit!(SealMinted {
        owner,
        seal_id,
        signer: seal.signer,
        expiry: seal.expiry,
    });
    Ok(())
}

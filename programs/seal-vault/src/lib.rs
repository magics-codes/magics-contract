//! # seal-vault
//!
//! The boundary store. Every seal is a written limit on an ephemeral key: which
//! program it may call, which instruction on that program, how much value per
//! call, how much per day, and until when. Verifying a cast routes through here
//! — and so does revocation.
//!
//! Bound to a single router at `initialize`. Verification mutates a seal (nonce
//! and the daily window), so only the bound router's cast-authority PDA may
//! advance state; anyone may still read.

use anchor_lang::prelude::*;

pub mod constants;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("Hs1WNEErp5rqhy8JvofPzL8UYHFaWj9gRDjofhNqQvhZ");

#[program]
pub mod seal_vault {
    use super::*;

    /// Bind the vault to a router program. One-time.
    pub fn initialize(ctx: Context<Initialize>, router_program: Pubkey) -> Result<()> {
        instructions::initialize::handler(ctx, router_program)
    }

    /// Mint or re-issue a seal under the caller.
    pub fn mint_seal(ctx: Context<MintSeal>, seal_id: [u8; 32], args: SealArgs) -> Result<()> {
        instructions::mint_seal::handler(ctx, seal_id, args)
    }

    /// Revoke a single seal with a free-form reason.
    pub fn revoke(ctx: Context<Revoke>, seal_id: [u8; 32], reason: String) -> Result<()> {
        instructions::revoke::handler(ctx, seal_id, reason)
    }

    /// Revoke every live seal the caller owns from `remaining_accounts`.
    pub fn revoke_all(ctx: Context<RevokeAll>) -> Result<()> {
        instructions::revoke_all::handler(ctx)
    }

    /// Verify a cast against a seal and consume one nonce. CPI-only, gated to
    /// the bound router's cast authority.
    pub fn verify_and_consume(
        ctx: Context<VerifyAndConsume>,
        seal_id: [u8; 32],
        agent_id: [u8; 32],
        deadline: i64,
        data_hash: [u8; 32],
        call_value: u64,
    ) -> Result<()> {
        instructions::verify::handler(ctx, seal_id, agent_id, deadline, data_hash, call_value)
    }
}

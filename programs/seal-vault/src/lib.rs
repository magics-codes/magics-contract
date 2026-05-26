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

declare_id!("Chydhx9FZ7dYwvNoRzbE8VMNHiQCJ8xWiydRuSoY9Q7W");

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
}

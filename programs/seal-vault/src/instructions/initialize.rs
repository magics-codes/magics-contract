use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::state::Config;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,
}

/// Bind the vault to one router program. Callable once — the config PDA can only
/// be initialised a single time.
pub fn handler(ctx: Context<Initialize>, router_program: Pubkey) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.router_program = router_program;
    config.bump = ctx.bumps.config;
    Ok(())
}

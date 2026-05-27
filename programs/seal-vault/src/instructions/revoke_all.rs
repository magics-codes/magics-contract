use anchor_lang::prelude::*;
use magics_common::MagicsError;

use crate::events::SealAllRevoked;
use crate::state::SealRecord;

/// The kill switch. The EVM `revokeAll()` iterated an on-chain list of the
/// owner's seals; on Solana there's no such list to walk, so the caller passes
/// the seal records as `remaining_accounts` and we revoke every live one that
/// belongs to them. Anything else in the list is skipped, not trusted.
#[derive(Accounts)]
pub struct RevokeAll<'info> {
    pub owner: Signer<'info>,
}

pub fn handler(ctx: Context<RevokeAll>) -> Result<()> {
    let owner = ctx.accounts.owner.key();
    let mut count: u64 = 0;

    for acc in ctx.remaining_accounts.iter() {
        // Only this program's writable accounts are candidates.
        if acc.owner != &crate::ID || !acc.is_writable {
            continue;
        }
        let mut data = acc.try_borrow_mut_data()?;
        let mut record = match SealRecord::try_deserialize(&mut &data[..]) {
            Ok(r) => r,
            Err(_) => continue, // not a seal record — leave it alone
        };
        if record.owner != owner || record.revoked {
            continue;
        }

        record.revoked = true;
        record.revoke_reason = "revoke-all".to_string();
        let mut cursor: &mut [u8] = &mut data;
        record.try_serialize(&mut cursor)?;
        count = count.checked_add(1).ok_or(MagicsError::NumericalOverflow)?;
    }

    emit!(SealAllRevoked { owner, count });
    Ok(())
}

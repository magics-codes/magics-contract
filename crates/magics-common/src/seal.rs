use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;

/// A seal is a written boundary around a session key. Whoever holds the secret
/// for `signer` may perform exactly the actions described by the rest of the
/// struct — nothing else, never past `expiry`.
///
/// The EVM seal pinned an `address` target and a `bytes4` selector; on Solana
/// the target is a program id and the selector is the 8-byte Anchor instruction
/// discriminator. `[0u8; 8]` is the wildcard, same spirit as `bytes4(0)`.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seal {
    /// Ephemeral key that signs cast intents — the only key the vault will
    /// accept an Ed25519 signature from.
    pub signer: Pubkey,
    /// Program the signer may target (the router, usually).
    pub target: Pubkey,
    /// Instruction discriminator on `target`. `[0u8; 8]` matches every one.
    pub selector: [u8; 8],
    /// Max token amount moved per individual call.
    pub value_cap: u64,
    /// Max token amount moved within any rolling 24h window.
    pub daily_cap: u64,
    /// Unix seconds after which the seal is dead. Hard wall, no extension.
    pub expiry: i64,
    /// Unix seconds at mint. Set by the vault, not the caller.
    pub created_at: i64,
    /// Arbitrary opaque tag — usually the target strategy's scope hash.
    pub scope_hash: [u8; 32],
}

/// Domain tag folded into the seal id so ids from this protocol never collide
/// with a raw keccak of the same bytes somewhere else.
pub const SEAL_TAG: &[u8] = b"magics:seal:v1";

impl Seal {
    /// Deterministic id for a seal under `owner`. Two seals with identical
    /// boundary parameters and owner collide — which is fine, they're
    /// interchangeable. Hashes the boundary fields only: `created_at` is set
    /// on-chain at mint, so folding it in would make the id — and therefore the
    /// record's PDA — unpredictable to the client that has to name it.
    pub fn id(&self, owner: &Pubkey) -> [u8; 32] {
        keccak::hashv(&[
            SEAL_TAG,
            owner.as_ref(),
            self.signer.as_ref(),
            self.target.as_ref(),
            &self.selector,
            &self.value_cap.to_le_bytes(),
            &self.daily_cap.to_le_bytes(),
            &self.expiry.to_le_bytes(),
            &self.scope_hash,
        ])
        .0
    }

    /// True if `now` falls inside the seal's lifetime window.
    pub fn is_live(&self, now: i64) -> bool {
        now < self.expiry && now >= self.created_at
    }

    /// True if `selector` is permitted. Wildcard (`[0u8; 8]`) matches anything;
    /// otherwise an exact match is required.
    pub fn permits_selector(&self, selector: &[u8; 8]) -> bool {
        self.selector == [0u8; 8] || &self.selector == selector
    }
}

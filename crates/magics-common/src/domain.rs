use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;

/// Domain pieces folded into every signed cast. The EVM side used an EIP-712
/// domain of `("magics", "1", chainId, verifyingContract)`; Ed25519 has no
/// notion of a typed domain, so we bind the same things into the message by
/// hand — protocol name, version, and the verifying program id stand in for
/// `verifyingContract`, which keeps a signature from one deployment from
/// replaying against another.
pub const DOMAIN_NAME: &[u8] = b"magics";
pub const DOMAIN_VERSION: &[u8] = b"1";
pub const CAST_TAG: &[u8] = b"magics:cast:v1";

/// keccak over the raw cast payload — the Solana analog of `keccak256(data)`
/// the router hashes before handing control to a strategy.
pub fn data_hash(data: &[u8]) -> [u8; 32] {
    keccak::hash(data).0
}

/// The exact bytes a session key signs before the router will route a call.
/// Mirrors the EIP-712 `Cast(bytes32 agentId,uint64 nonce,uint64 deadline,
/// bytes32 dataHash)` leaf: domain first, then the four fields in a fixed
/// order. Off-chain signers must reproduce these bytes byte-for-byte.
pub fn cast_message(
    vault_program: &Pubkey,
    agent_id: &[u8; 32],
    nonce: u64,
    deadline: i64,
    data_hash: &[u8; 32],
) -> Vec<u8> {
    let mut m = Vec::with_capacity(
        CAST_TAG.len() + DOMAIN_NAME.len() + DOMAIN_VERSION.len() + 32 + 32 + 8 + 8 + 32,
    );
    m.extend_from_slice(CAST_TAG);
    m.extend_from_slice(DOMAIN_NAME);
    m.extend_from_slice(DOMAIN_VERSION);
    m.extend_from_slice(vault_program.as_ref());
    m.extend_from_slice(agent_id);
    m.extend_from_slice(&nonce.to_le_bytes());
    m.extend_from_slice(&deadline.to_le_bytes());
    m.extend_from_slice(data_hash);
    m
}

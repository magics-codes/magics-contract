//! # magics-common
//!
//! The pieces every magics program shares: the error catalogue, the seal and
//! cast types, the deterministic id derivations, and the Ed25519 helper that
//! checks a session key actually signed a cast. Lives in one crate so the wire
//! format — error codes, hashes, the signed message — is defined exactly once.

pub mod domain;
pub mod ed25519;
pub mod error;
pub mod seal;

pub use domain::{cast_message, data_hash};
pub use ed25519::verify_cast_signature;
pub use error::MagicsError;
pub use seal::Seal;

/// PDA seed for the router's cast authority. The router signs `verify_and_consume`
/// (and the strategy budget calls) with the PDA at `[CAST_AUTHORITY_SEED]` under
/// its own program id; the vault checks that signer to know the call really came
/// from the router it was bound to. The Solana stand-in for the EVM vault's
/// `msg.sender == router`.
pub const CAST_AUTHORITY_SEED: &[u8] = b"cast-authority";

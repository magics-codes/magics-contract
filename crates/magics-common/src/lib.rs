//! # magics-common
//!
//! The pieces every magics program shares: the error catalogue, the seal and
//! cast types, the deterministic id derivations, and the Ed25519 helper that
//! checks a session key actually signed a cast. Lives in one crate so the wire
//! format — error codes, hashes, the signed message — is defined exactly once.

pub mod error;

pub use error::MagicsError;

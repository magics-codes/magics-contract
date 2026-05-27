use anchor_lang::prelude::*;

/// Centralised error catalogue so every program reverts against the same set of
/// codes. Anchor numbers these from 6000 upward in declaration order — keeping
/// them in one place means the SDK decodes one stable surface no matter which
/// program threw. Append only; never reorder, or you renumber the wire format.
#[error_code]
pub enum MagicsError {
    // ── Access ──────────────────────────────────────────────────────────────
    #[msg("caller is not the owner")]
    NotOwner,
    #[msg("caller is not the seal vault")]
    NotSealVault,
    #[msg("caller is not the router")]
    NotRouter,
    #[msg("caller is not the active strategy")]
    NotStrategy,

    // ── Seal lifecycle ──────────────────────────────────────────────────────
    #[msg("seal does not exist")]
    SealUnknown,
    #[msg("supplied seal id does not match its parameters")]
    SealIdMismatch,
    #[msg("seal has expired")]
    SealExpired,
    #[msg("seal is revoked")]
    SealRevoked,
    #[msg("revoke reason exceeds the maximum length")]
    ReasonTooLong,
    #[msg("seal value or daily cap breached")]
    SealCapBreached,
    #[msg("seal does not permit this target")]
    SealTargetMismatch,
    #[msg("cast nonce replays a consumed nonce")]
    SealNonceReplay,
    #[msg("cast signature does not verify against the seal signer")]
    SealSignatureInvalid,

    // ── Agent lifecycle ─────────────────────────────────────────────────────
    #[msg("agent does not exist")]
    AgentUnknown,
    #[msg("agent is not active")]
    AgentNotActive,
    #[msg("agent name is empty")]
    AgentNameEmpty,
    #[msg("agent name exceeds the maximum length")]
    AgentNameTooLong,

    // ── Router / cast ───────────────────────────────────────────────────────
    #[msg("cast deadline has passed")]
    CastDeadlinePassed,
    #[msg("no cast is active")]
    NoActiveCast,
    #[msg("active-cast context does not match this agent or strategy")]
    CastContextMismatch,
    #[msg("strategy execution failed")]
    StrategyCallFailed,
    #[msg("token mint does not match")]
    MintMismatch,
    #[msg("amount must be non-zero")]
    AmountZero,
    #[msg("insufficient agent balance")]
    InsufficientBalance,

    // ── Signature verification ──────────────────────────────────────────────
    #[msg("expected an Ed25519 verification instruction was not found")]
    MissingEd25519Instruction,
    #[msg("Ed25519 instruction is malformed")]
    MalformedEd25519Instruction,

    // ── Generic ─────────────────────────────────────────────────────────────
    #[msg("unexpected zero address")]
    ZeroAddress,
    #[msg("account already initialised")]
    AlreadyInitialised,
    #[msg("numerical overflow")]
    NumericalOverflow,
}

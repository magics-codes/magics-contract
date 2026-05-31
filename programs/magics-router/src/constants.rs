/// PDA seed prefix for a per-agent, per-mint balance: `[BALANCE_SEED, agent_id, mint]`.
pub const BALANCE_SEED: &[u8] = b"balance";

/// PDA seed prefix for a vault token account: `[VAULT_TOKEN_SEED, mint]`.
pub const VAULT_TOKEN_SEED: &[u8] = b"vault";

/// PDA seed for the single authority that owns every vault token account.
pub const VAULT_AUTH_SEED: &[u8] = b"vault-auth";

/// PDA seed prefix for the transient active-cast context: `[ACTIVE_SEED, agent_id]`.
pub const ACTIVE_SEED: &[u8] = b"active";

/// PDA seed for the identity a strategy program signs budget calls with. The
/// router checks `[STRATEGY_AUTH_SEED]` under the active strategy's program id.
pub const STRATEGY_AUTH_SEED: &[u8] = b"strategy";

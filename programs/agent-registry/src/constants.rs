/// PDA seed prefix for an agent record: `[AGENT_SEED, agent_id]`.
pub const AGENT_SEED: &[u8] = b"agent";

/// PDA seed prefix for an owner's monotonic summon counter.
pub const COUNTER_SEED: &[u8] = b"counter";

/// Domain tag folded into a derived agent id.
pub const AGENT_TAG: &[u8] = b"magics:agent:v1";

/// Cap on agent names. 64 bytes is room for "long-tail-sol-yield-bot-v3" and
/// then some — matches the EVM registry's `MAX_NAME_LENGTH`.
pub const MAX_NAME_LENGTH: usize = 64;

/// PDA seed for the singleton config that binds this vault to one router.
pub const CONFIG_SEED: &[u8] = b"config";

/// PDA seed prefix for a seal record: `[SEAL_SEED, seal_id]`.
pub const SEAL_SEED: &[u8] = b"seal";

/// Free-form revoke reason cap. Long reasons cost rent and earn nothing.
pub const MAX_REASON_LEN: usize = 64;

/// Rolling daily-cap window, in seconds. Mirrors the EVM `_WINDOW = 1 days`.
pub const WINDOW: i64 = 86_400;

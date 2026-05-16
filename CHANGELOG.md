# Changelog

All notable changes to the magics contracts. Versions follow semver — the
on-chain ABI is part of the public surface, so a breaking ABI change is a major
bump.

## [0.1.0] — 2026-05-19

The first cut. Verified on Base Sepolia.

### Added
- `SealVault` — session-key boundary store with EIP-712 verification, rolling
  daily caps, nonce-tracked replay protection, and `revokeAll` kill switch.
- `AgentRegistry` — owner-keyed agent index with status lifecycle
  (Active / Paused / Halted) and seal rotation.
- `MagicsRouter` — per-agent ERC20 ledger + the single `cast` entry point.
  Active-cast context held in transient storage; strategies authorise budget
  pulls via the slot, not via msg.sender from the strategy.
- `PassiveYieldStrategy` — reference strategy: compound idle ERC20 into a
  yield vault, harvest back. Two ops, two leaves.
- `Errors` library — all reverts as 4-byte custom errors with named fields.
- Deploy + verify scripts.

### Notes
- Solc 0.8.26, Cancun. Uses `tstore` / `tload` directly — no transient keyword
  required.
- No external dependencies beyond `forge-std`. Ownable / ECDSA / EIP-712 /
  SafeTransfer are all inlined under `src/utils/` to keep the deployed
  bytecode self-contained for verification.

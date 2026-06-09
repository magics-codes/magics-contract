# Changelog

All notable changes to the magics Solana programs. Versions follow semver — the
on-chain IDL is part of the public surface, so a breaking instruction or account
change is a major bump.

## [0.1.0] — 2026-06-10

The first cut of the Solana port. Verified on devnet.

### Added
- `seal-vault` — session-key boundary store. Seal records as PDAs, Ed25519 cast
  verification through the instructions sysvar, rolling daily caps, nonce-tracked
  replay protection, and a `revoke_all` kill switch over `remaining_accounts`.
- `agent-registry` — owner-keyed agent index with a status lifecycle
  (Active / Paused / Halted) and seal rotation. Agent ids derived from
  (owner, strategy, seal, counter).
- `magics-router` — per-agent SPL ledger over a shared vault, the single `cast`
  entry point, and the strategy budget API (`pull` / `push`). The transient
  cast context is a short-lived PDA, opened after the seal verifies and closed
  when the cast returns.
- `passive-yield` — reference strategy: compound idle balance into a yield vault,
  harvest back. Two ops, two leaves.
- `mock-yield` — a toy 4626-like vault for tests.
- `magics-common` — shared seal type, id derivations, cast message, Ed25519
  helper, and the error catalogue.

### Notes
- Anchor 0.31.1, Solana 3.x. Program wiring is fixed at compile time via
  `declare_id!` + crate deps; the only runtime binding is the vault → router
  link set once at `initialize`.
- No transient storage on Solana — the EVM router's `tstore` slot becomes the
  `ActiveCast` PDA, and the EVM EIP-712 + ECDSA path becomes a canonical signed
  message checked against the Ed25519 native program.
- This is a reference implementation. It is not audited. Use it on devnet, read
  the code, file issues.

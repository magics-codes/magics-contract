<div align="center">

<img src="assets/banner.png" alt="magics solana programs" width="100%" />

# magics-contract-solana

**The Solana-side primitives that power the magics protocol.**
Session keys, an agent registry, a router, and a reference strategy — all in
Anchor 0.31, all under MIT.

[![Anchor 0.31](https://img.shields.io/badge/anchor-0.31.1-512da8.svg?style=flat-square)](./Anchor.toml)
[![Solana](https://img.shields.io/badge/solana-3.x-14f195.svg?style=flat-square)](https://solana.com)
[![Tests](https://img.shields.io/badge/tests-anchor-success.svg?style=flat-square)](./tests)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](./LICENSE)

</div>

---

This is the Solana port of [`magics-contract`](https://github.com/magics-codes/magics-contract).
The shape is the same — a boundary you can mint, read, and kill — translated
from Solidity into accounts and CPI.

## The shape of it

```
┌────────────┐        ┌──────────────┐        ┌──────────────┐
│   owner    │─mint──▶│  seal-vault  │◀─read──│   anyone     │
└────────────┘        └──────┬───────┘        └──────────────┘
       │                     │ verify_and_consume (CPI)
       │ summon              ▼
       ▼              ┌──────────────┐  CPI   ┌──────────────┐
┌──────────────┐      │magics-router │──exec─▶│ passive-yield│
│agent-registry│◀read─┤              │◀pull/──┤  (strategy)  │
└──────────────┘      └──────────────┘  push  └──────────────┘
```

Four programs, four jobs:

| Program            | What it holds                                              | What it lets you do                              |
| :----------------- | :--------------------------------------------------------- | :----------------------------------------------- |
| `seal-vault`       | Session-key boundaries: signer, target, caps, expiry, nonce | `mint_seal` / `revoke` / `revoke_all` / `verify_and_consume` |
| `agent-registry`   | (owner, strategy, seal, name) tuples under a stable id      | `summon` / `pause` / `resume` / `halt` / `rotate_seal` |
| `magics-router`    | Per-agent SPL ledger + the cast entry point                | `deposit` / `withdraw` / `cast` / `pull` / `push` |
| `passive-yield`    | Reference strategy — compound idle deposits, harvest yield  | `execute(op, amount)` — `op ∈ {compound, harvest}` |

`seal-vault` is bound to one router at `initialize`; only that router's
cast-authority PDA may advance seal state. `magics-router` keeps the active-cast
context in a **transient PDA** so a strategy can `pull` / `push` without
re-passing the agent — and the account is closed when the cast returns. The
registry holds no funds and runs no code; it just names a triple.

## Quickstart

```bash
# 1. Install the Solana + Anchor toolchain (anchor 0.31, solana 3.x).
#    https://www.anchor-lang.com/docs/installation

# 2. Build every program (compiles to SBF, emits IDLs under target/idl).
anchor build

# 3. Test. Spins up a local validator, deploys, runs the TS suite.
#    On Windows the validator needs Developer Mode (it creates symlinks).
anchor test

# 4. Deploy + wire to devnet.
anchor deploy --provider.cluster devnet
anchor migrate --provider.cluster devnet     # binds seal-vault → router
npx ts-node scripts/record-deployment.ts devnet
```

Program ids end up in `deployments/{cluster}.json`. Both the CLI and the web app
read from there at start-up.

## How the EVM maps to Solana

The Solidity contracts assumed things Solana doesn't have. The port keeps the
guarantees and swaps the machinery:

| EVM                                   | Solana                                                    |
| :------------------------------------ | :-------------------------------------------------------- |
| `mapping(bytes32 => Record)`          | one PDA per record, seeded by the id                      |
| EIP-712 typed data + `ECDSA.recover`  | a canonical signed message + the Ed25519 native program checked via the instructions sysvar |
| transient storage (`tstore`/`tload`)  | a short-lived `ActiveCast` PDA, opened and closed inside `cast` |
| `msg.sender == router`                | the router signs the verify CPI with its `[cast-authority]` PDA |
| `IStrategy(addr).execute(...)`        | a raw `execute` CPI (Anchor discriminator) into the agent's strategy program |
| strategy authorises pulls via slot    | the strategy signs `pull`/`push` with its `[strategy]` PDA, matched against the open cast |
| ERC-20                                | SPL Token; funds pool in a vault token account, the ledger is `Balance` PDAs |

## Module tour

### `seal-vault` — the boundary store

A seal is a written limit on an ephemeral key. The vault stores the parameters
and the usage counters in a PDA seeded by a deterministic `seal_id` — the keccak
of the owner and the boundary fields. A cast is verified here: the session key
signs a canonical message off-chain, the transaction carries an Ed25519 verify
instruction, and `verify_and_consume` confirms it covers the right signer and
message, then bumps the nonce and the rolling daily window. `revoke_all` halts
every seal you pass in one instruction — the kill switch.

### `agent-registry` — the book of names

Every agent has an id derived from `(owner, strategy, seal, counter)`. The
registry never moves funds; it records who acts as whom, lets owners
pause / resume / halt, and rotates the seal under a live agent.

### `magics-router` — the hinge

Owners use `deposit`, `withdraw`, `cast`. A cast is one transaction: the router
checks the agent is active, CPIs `verify_and_consume`, opens the `ActiveCast`
PDA, hands control to the strategy, and closes the context. During `execute` the
strategy sees `pull` and `push`, both gated on the open context plus the
strategy's own PDA signature — it can't touch a budget that isn't its agent's,
and it can't act outside a seal-verified cast.

### `passive-yield` — the reference body

Two ops. `0x01` compounds idle SPL into a yield vault; `0x02` redeems shares and
credits the result back to the agent's router balance. Use it as the template
when you write your own strategy: implement `execute`, sign your budget calls
with `[strategy]`, never trust a caller.

## Layout

```
contracts-solana/
├── programs/
│   ├── seal-vault/          # session-key boundary store
│   ├── agent-registry/      # owner ↔ agent index
│   ├── magics-router/       # the hinge — ledger + cast entry
│   ├── passive-yield/       # reference strategy
│   └── mock-yield/          # toy 4626-like vault for tests
├── crates/
│   └── magics-common/       # seal type, ids, cast message, ed25519, errors
├── tests/                   # anchor ts-mocha suite
├── migrations/deploy.ts     # post-deploy wiring
├── scripts/                 # deployment manifest writer
├── deployments/             # written per cluster
├── Anchor.toml
└── Cargo.toml
```

## Security model in one paragraph

A magics agent has authority equal to its seal, never more. The seal is on
chain, anyone-readable, mutable only by its owner. A strategy is a deterministic
function over the agent's ledger inside the router — it cannot reach outside the
budget the router exposes, because the budget API is gated on a transient PDA the
router controls and a program signature only the strategy can produce. The kill
switch (`revoke_all`) is one transaction from the owner's root key.

## Deployments

| Cluster | Manifest                                       |
| :------ | :--------------------------------------------- |
| devnet  | [`deployments/devnet.json`](./deployments/devnet.json) |

Program ids are the deploy keypairs' public keys, so they're identical on every
cluster — the manifest just records where they were last pushed.

## Audit status

This is the **v0.1.0 reference port**. It is not audited. Use it on devnet, read
the code, file issues. We will list audit reports here once they exist — not
before.

## License

MIT. See [LICENSE](./LICENSE).

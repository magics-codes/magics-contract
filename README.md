<div align="center">

<img src="assets/banner.png" alt="magics contracts" width="100%" />

# magics-contracts

**The Base-side primitives that power the magics protocol.**
Session keys, agent registry, router, and a reference strategy — all in Solidity 0.8.26, all under MIT.

[![Solidity 0.8.26](https://img.shields.io/badge/solidity-0.8.26-555.svg?style=flat-square)](./foundry.toml)
[![Foundry](https://img.shields.io/badge/built%20with-foundry-orange.svg?style=flat-square)](https://book.getfoundry.sh)
[![Tests](https://img.shields.io/badge/tests-passing-success.svg?style=flat-square)](./test)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](./LICENSE)

</div>

---

## The shape of it

```
┌────────────┐          ┌──────────────┐         ┌──────────────┐
│   owner    │──mint───▶│  SealVault   │◀──read──│   anyone     │
└────────────┘          └──────┬───────┘         └──────────────┘
       │                       │ verifyAndConsume
       │ summon                ▼
       ▼                ┌──────────────┐         ┌──────────────┐
┌──────────────┐        │MagicsRouter  │──exec──▶│  IStrategy   │
│AgentRegistry │◀──read─┤              │◀─pull──┤ (your code)  │
└──────────────┘        └──────────────┘  push  └──────────────┘
```

Four contracts, four jobs:

| Contract               | What it holds                                       | What it lets you do                                 |
| :--------------------- | :-------------------------------------------------- | :-------------------------------------------------- |
| `SealVault`            | Session-key boundaries: target, selector, caps, expiry | mint / revoke / verifyAndConsume                 |
| `AgentRegistry`        | (owner, strategy, seal, name) tuples with a stable id  | summon / pause / resume / halt / rotateSeal      |
| `MagicsRouter`         | Per-agent ERC20 ledger + the cast entry point          | deposit / withdraw / cast / pull / push          |
| `PassiveYieldStrategy` | Reference strategy — compound idle deposits, harvest yield | execute(op, amount) — `op ∈ {compound, harvest}` |

`SealVault` is bound to a single router at construction. `MagicsRouter` keeps active-cast context in **transient storage** so a strategy can call `pull` / `push` without re-passing the agent id, and the slot evaporates at end of tx. `AgentRegistry` holds no funds and runs no code — it just gives a name to a triple.

## Quickstart

```bash
# 1. Install Foundry if you haven't.
curl -L https://foundry.paradigm.xyz | bash && foundryup

# 2. Pull forge-std.
forge install foundry-rs/forge-std --no-commit

# 3. Build + test.
forge build
forge test -vv

# 4. Deploy to Base Sepolia.
cp .env.example .env  # fill DEPLOYER_PRIVATE_KEY + BASESCAN_API_KEY
forge script script/Deploy.s.sol \
  --rpc-url base_sepolia \
  --broadcast --verify \
  --etherscan-api-key $BASESCAN_API_KEY
```

Addresses end up in `deployments/{chainId}.json`. Both the CLI and the web app read from there at start-up.

## Module tour

### `SealVault.sol` — the boundary store

A seal is a written limit on an ephemeral key. The vault stores the parameters and the usage counters; everything is keyed by a deterministic `sealId = keccak256(owner ‖ Seal struct)`.

```solidity
struct Seal {
    address signer;       // ephemeral key — the only address that may sign casts
    address target;       // contract the signer may call (the router, usually)
    bytes4  selector;     // function on `target`; bytes4(0) = wildcard
    uint128 valueCap;     // max ETH value per individual call
    uint128 dailyCap;     // max total ETH value within any rolling 24h
    uint64  expiry;       // unix seconds — hard wall, no extension
    uint64  createdAt;    // set by the vault at mint
    bytes32 scopeHash;    // opaque tag, usually the target strategy's scope
}
```

Signatures use EIP-712 with `domain = ("magics", "1", chainId, vault)` and:

```
Cast(bytes32 agentId, uint64 nonce, uint64 deadline, bytes32 dataHash)
```

Revoke is single-line: `vault.revoke(sealId, "reason")`. `revokeAll()` halts every active seal under one wallet in one transaction — the kill switch.

### `AgentRegistry.sol` — the book of names

Every agent has an id derived from `(chainId, owner, strategy, sealId, counter)`. The registry never moves funds; it just records who is supposed to be acting as whom, lets owners pause / resume / halt, and supports rotating the seal under an existing agent without re-summoning.

### `MagicsRouter.sol` — the hinge

Owners interact through three calls: `deposit`, `withdraw`, `cast`. Strategies, during their `execute` call, see two more: `pull` and `push`, both gated on the transient active-strategy slot.

A cast is one transaction:

1. Caller submits `cast(agentId, deadline, data, sig)`.
2. Router looks up the agent and its seal.
3. Vault verifies the EIP-712 signature against the seal's `signer`, applies caps, bumps the nonce.
4. Router stores `(agentId, strategy)` into transient storage and delegates to `IStrategy.execute`.
5. The strategy pulls / pushes against the agent's ledger as needed.
6. Router clears the transient slot and emits `Cast`.

### `strategies/PassiveYieldStrategy.sol` — the reference body

Two ops, two leaves. `0x01` compounds idle ERC20 into a yield vault. `0x02` redeems shares and credits the result back to the agent's router balance. Use this as the template when writing your own `IStrategy`.

## Layout

```
contracts/
├── src/
│   ├── SealVault.sol               # session-key boundary store
│   ├── AgentRegistry.sol           # owner ↔ agent index
│   ├── MagicsRouter.sol            # the hinge — ledger + cast entry
│   ├── interfaces/
│   │   ├── ISealVault.sol
│   │   ├── IAgentRegistry.sol
│   │   ├── IMagicsRouter.sol
│   │   ├── IStrategy.sol
│   │   └── IERC20.sol
│   ├── libraries/
│   │   ├── SealLib.sol             # seal struct, typehashes, helpers
│   │   └── Errors.sol              # 4-byte custom error catalogue
│   ├── utils/
│   │   ├── Ownable.sol             # two-step ownership
│   │   ├── ReentrancyGuard.sol     # transient-slot guard
│   │   ├── EIP712.sol              # cached domain separator
│   │   ├── ECDSA.sol               # recovery + EIP-2098 compact sigs
│   │   └── SafeTransferLib.sol     # ERC20 transfer helpers
│   ├── strategies/
│   │   └── PassiveYieldStrategy.sol
│   └── mocks/
│       ├── MockERC20.sol
│       └── MockYieldVault.sol
├── test/
│   ├── Base.t.sol                  # shared scaffolding
│   ├── SealVault.t.sol
│   ├── AgentRegistry.t.sol
│   ├── MagicsRouter.t.sol
│   └── PassiveYieldStrategy.t.sol
├── script/
│   ├── Deploy.s.sol                # core deployment
│   ├── DeployStrategy.s.sol        # strategy add-ons
│   └── Verify.s.sol                # prints `forge verify-contract` commands
├── deployments/                    # written by Deploy.s.sol
├── foundry.toml
├── remappings.txt
└── README.md
```

## Security model in one paragraph

A magics agent has authority equal to its seal, never more. The seal is on-chain, anyone-readable, mutable only by its owner. The strategy code is a deterministic function over the agent's ledger inside the router — it cannot reach outside the budget the router exposes, because the budget API is gated on a transient slot the router controls. The kill switch (`vault.revokeAll`) is one transaction from the owner's root signer.

The threat model and recommended hardening are written up in [`docs/protocol/security-model.md`](../docs/protocol/security-model.md).

## Audit status

This is the **v0.1.0 reference implementation**. It is not audited. Use it on Base Sepolia, read the code, file issues. We will list audit reports here once they exist — not before.

## License

MIT. See [LICENSE](./LICENSE).

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IMagicsRouter} from "./interfaces/IMagicsRouter.sol";
import {ISealVault} from "./interfaces/ISealVault.sol";
import {IAgentRegistry} from "./interfaces/IAgentRegistry.sol";
import {IStrategy} from "./interfaces/IStrategy.sol";
import {Errors} from "./libraries/Errors.sol";
import {SafeTransferLib} from "./utils/SafeTransferLib.sol";
import {ReentrancyGuard} from "./utils/ReentrancyGuard.sol";

/// @title MagicsRouter
/// @notice The hinge. Every cast goes through here. Holds the per-agent token
///         ledger, validates the seal before delegating to the strategy, and
///         opens a transient budget the strategy can spend from.
///
/// @dev    The active-cast context is held in transient storage so it survives
///         the strategy.execute(...) call but vanishes at end of tx. Strategies
///         call back into pull() / push() with no further arguments — they
///         can't lie about which agent's budget they're touching.
contract MagicsRouter is IMagicsRouter, ReentrancyGuard {
    ISealVault public immutable vault;
    IAgentRegistry public immutable registry;

    /// @dev agentId => token => balance
    mapping(bytes32 => mapping(address => uint256)) internal _balances;

    // Transient slots — keccak("magics.router.active.agent") / .strategy
    bytes32 private constant _ACTIVE_AGENT_SLOT =
        0x6f1f9b9b8f3f7e2d4c5b6a7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f;
    bytes32 private constant _ACTIVE_STRATEGY_SLOT =
        0x7e2c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c;

    constructor(address vault_, address registry_) {
        if (vault_ == address(0) || registry_ == address(0)) revert Errors.ZeroAddress();
        vault = ISealVault(vault_);
        registry = IAgentRegistry(registry_);
    }

    // ── Owner-facing motions ──────────────────────────────────────────────────

    /// @inheritdoc IMagicsRouter
    function deposit(bytes32 agentId, address token, uint256 amount) external nonReentrant {
        if (token == address(0)) revert Errors.TokenZero();
        if (amount == 0) revert Errors.AmountZero();

        IAgentRegistry.Agent memory a = registry.agentOf(agentId);
        if (a.owner != msg.sender) revert Errors.NotOwner(msg.sender);

        SafeTransferLib.safeTransferFrom(token, msg.sender, address(this), amount);
        _balances[agentId][token] += amount;
        emit Deposited(agentId, token, amount);
    }

    /// @inheritdoc IMagicsRouter
    function withdraw(bytes32 agentId, address token, uint256 amount, address to)
        external
        nonReentrant
    {
        if (token == address(0)) revert Errors.TokenZero();
        if (amount == 0) revert Errors.AmountZero();
        if (to == address(0)) revert Errors.ZeroAddress();

        IAgentRegistry.Agent memory a = registry.agentOf(agentId);
        if (a.owner != msg.sender) revert Errors.NotOwner(msg.sender);

        uint256 bal = _balances[agentId][token];
        if (bal < amount) revert Errors.InsufficientBalance(bal, amount);

        unchecked {
            _balances[agentId][token] = bal - amount;
        }
        SafeTransferLib.safeTransfer(token, to, amount);
        emit Withdrawn(agentId, token, amount);
    }

    // ── Cast entry point ──────────────────────────────────────────────────────

    /// @inheritdoc IMagicsRouter
    function cast(
        bytes32 agentId,
        uint64 deadline,
        bytes calldata data,
        bytes calldata signature
    ) external nonReentrant returns (bytes memory result) {
        if (block.timestamp > deadline) revert Errors.CastDeadlinePassed(deadline);

        IAgentRegistry.Agent memory a = registry.agentOf(agentId);
        if (a.status != IAgentRegistry.Status.Active) {
            revert Errors.AgentNotActive(agentId, uint8(a.status));
        }

        bytes32 dataHash = keccak256(data);
        uint64 nonce =
            vault.verifyAndConsume(a.seal, agentId, deadline, dataHash, 0, signature);

        _setActive(agentId, a.strategy);
        try IStrategy(a.strategy).execute(agentId, a.owner, data) returns (bytes memory r) {
            result = r;
        } catch (bytes memory reason) {
            _clearActive();
            revert Errors.StrategyCallFailed(reason);
        }
        _clearActive();

        emit Cast(agentId, a.seal, a.strategy, nonce, dataHash);
    }

    // ── Strategy-facing budget API ────────────────────────────────────────────

    /// @inheritdoc IMagicsRouter
    function pull(address token, uint256 amount) external {
        (bytes32 agentId, address active) = _readActive();
        if (msg.sender != active) revert Errors.NotStrategy(msg.sender);
        if (token == address(0)) revert Errors.TokenZero();
        if (amount == 0) revert Errors.AmountZero();

        uint256 bal = _balances[agentId][token];
        if (bal < amount) revert Errors.InsufficientBalance(bal, amount);

        unchecked {
            _balances[agentId][token] = bal - amount;
        }
        SafeTransferLib.safeTransfer(token, msg.sender, amount);
    }

    /// @inheritdoc IMagicsRouter
    function push(address token, address /*to*/, uint256 amount) external {
        // `to` is ignored — pushed tokens always credit the active agent.
        (bytes32 agentId, address active) = _readActive();
        if (msg.sender != active) revert Errors.NotStrategy(msg.sender);
        if (token == address(0)) revert Errors.TokenZero();
        if (amount == 0) revert Errors.AmountZero();

        SafeTransferLib.safeTransferFrom(token, msg.sender, address(this), amount);
        _balances[agentId][token] += amount;
    }

    // ── Views ─────────────────────────────────────────────────────────────────

    /// @inheritdoc IMagicsRouter
    function balanceOf(bytes32 agentId, address token) external view returns (uint256) {
        return _balances[agentId][token];
    }

    function activeContext() external view returns (bytes32 agentId, address strategy) {
        return _readActive();
    }

    // ── Internal transient helpers ────────────────────────────────────────────

    function _setActive(bytes32 agentId, address strategy) internal {
        bytes32 a = _ACTIVE_AGENT_SLOT;
        bytes32 s = _ACTIVE_STRATEGY_SLOT;
        assembly {
            tstore(a, agentId)
            tstore(s, strategy)
        }
    }

    function _clearActive() internal {
        bytes32 a = _ACTIVE_AGENT_SLOT;
        bytes32 s = _ACTIVE_STRATEGY_SLOT;
        assembly {
            tstore(a, 0)
            tstore(s, 0)
        }
    }

    function _readActive() internal view returns (bytes32 agentId, address strategy) {
        bytes32 a = _ACTIVE_AGENT_SLOT;
        bytes32 s = _ACTIVE_STRATEGY_SLOT;
        assembly {
            agentId := tload(a)
            strategy := tload(s)
        }
    }
}

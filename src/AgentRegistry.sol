// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IAgentRegistry} from "./interfaces/IAgentRegistry.sol";
import {ISealVault} from "./interfaces/ISealVault.sol";
import {IStrategy} from "./interfaces/IStrategy.sol";
import {Errors} from "./libraries/Errors.sol";

/// @title AgentRegistry
/// @notice The book of names. An agent record pairs an owner, a strategy, and
///         a seal — and gives the triple a stable id you can refer to from
///         anywhere else in the system.
///
/// @dev    The registry never holds funds and never executes anything. All it
///         does is bind names to (owner, strategy, seal) tuples and let owners
///         change the binding within the rules.
contract AgentRegistry is IAgentRegistry {
    /// @dev Cap on agent names. Long names cost more to log and don't earn
    ///      their gas back; 64 bytes is room for "long-tail-eth-yield-bot-v3"
    ///      and then some.
    uint256 public constant MAX_NAME_LENGTH = 64;

    ISealVault public immutable vault;

    mapping(bytes32 => Agent) internal _agents;
    mapping(address => bytes32[]) internal _idsByOwner;
    mapping(address => uint64) internal _counter;

    constructor(address vault_) {
        if (vault_ == address(0)) revert Errors.ZeroAddress();
        vault = ISealVault(vault_);
    }

    // ── Mutations ─────────────────────────────────────────────────────────────

    /// @inheritdoc IAgentRegistry
    function summon(string calldata name, address strategy, bytes32 sealId)
        external
        returns (bytes32 agentId)
    {
        bytes memory nameBytes = bytes(name);
        if (nameBytes.length == 0) revert Errors.AgentNameEmpty();
        if (nameBytes.length > MAX_NAME_LENGTH) revert Errors.AgentNameTooLong(nameBytes.length);
        if (strategy == address(0)) revert Errors.AgentStrategyZero();

        // Cross-check the seal exists and the caller owns it. We don't enforce
        // that the seal is currently un-revoked here — the router will catch
        // that at cast time, and we still want users to be able to register an
        // agent and rotate the seal later.
        if (vault.ownerOf(sealId) != msg.sender) revert Errors.NotOwner(msg.sender);

        // Touching `scopeHash` also serves as a soft proof that `strategy`
        // implements IStrategy — an EOA or wrong-ABI contract will revert here.
        IStrategy(strategy).scopeHash();

        uint64 nextIdx;
        unchecked {
            nextIdx = ++_counter[msg.sender];
        }

        agentId = keccak256(
            abi.encodePacked(block.chainid, msg.sender, strategy, sealId, nextIdx)
        );

        Agent memory a = Agent({
            id: agentId,
            owner: msg.sender,
            strategy: strategy,
            seal: sealId,
            createdAt: uint64(block.timestamp),
            updatedAt: uint64(block.timestamp),
            status: Status.Active,
            name: name
        });
        _agents[agentId] = a;
        _idsByOwner[msg.sender].push(agentId);

        emit AgentSummoned(msg.sender, agentId, strategy, sealId, name);
    }

    /// @inheritdoc IAgentRegistry
    function pause(bytes32 agentId, string calldata reason) external {
        Agent storage a = _agents[agentId];
        if (a.owner != msg.sender) revert Errors.NotOwner(msg.sender);
        Status prev = a.status;
        if (prev == Status.Halted) revert Errors.AgentNotActive(agentId, uint8(prev));
        a.status = Status.Paused;
        a.updatedAt = uint64(block.timestamp);
        emit AgentStatusChanged(agentId, prev, Status.Paused, reason);
    }

    /// @inheritdoc IAgentRegistry
    function resume(bytes32 agentId) external {
        Agent storage a = _agents[agentId];
        if (a.owner != msg.sender) revert Errors.NotOwner(msg.sender);
        Status prev = a.status;
        if (prev == Status.Halted) revert Errors.AgentNotActive(agentId, uint8(prev));
        a.status = Status.Active;
        a.updatedAt = uint64(block.timestamp);
        emit AgentStatusChanged(agentId, prev, Status.Active, "");
    }

    /// @inheritdoc IAgentRegistry
    function halt(bytes32 agentId, string calldata reason) external {
        Agent storage a = _agents[agentId];
        if (a.owner != msg.sender) revert Errors.NotOwner(msg.sender);
        Status prev = a.status;
        a.status = Status.Halted;
        a.updatedAt = uint64(block.timestamp);
        emit AgentStatusChanged(agentId, prev, Status.Halted, reason);
    }

    /// @inheritdoc IAgentRegistry
    function rotateSeal(bytes32 agentId, bytes32 newSeal) external {
        Agent storage a = _agents[agentId];
        if (a.owner != msg.sender) revert Errors.NotOwner(msg.sender);
        if (vault.ownerOf(newSeal) != msg.sender) revert Errors.NotOwner(msg.sender);
        bytes32 oldSeal = a.seal;
        a.seal = newSeal;
        a.updatedAt = uint64(block.timestamp);
        emit AgentSealRotated(agentId, oldSeal, newSeal);
    }

    // ── Views ─────────────────────────────────────────────────────────────────

    /// @inheritdoc IAgentRegistry
    function agentOf(bytes32 agentId) external view returns (Agent memory) {
        Agent memory a = _agents[agentId];
        if (a.owner == address(0)) revert Errors.AgentUnknown(agentId);
        return a;
    }

    /// @inheritdoc IAgentRegistry
    function agentsOf(address owner) external view returns (Agent[] memory out) {
        bytes32[] memory ids = _idsByOwner[owner];
        out = new Agent[](ids.length);
        for (uint256 i; i < ids.length; ++i) {
            out[i] = _agents[ids[i]];
        }
    }

    /// @inheritdoc IAgentRegistry
    function idsOf(address owner) external view returns (bytes32[] memory) {
        return _idsByOwner[owner];
    }

    /// @inheritdoc IAgentRegistry
    function isActive(bytes32 agentId) external view returns (bool) {
        return _agents[agentId].status == Status.Active;
    }

    /// @inheritdoc IAgentRegistry
    function exists(bytes32 agentId) external view returns (bool) {
        return _agents[agentId].owner != address(0);
    }
}

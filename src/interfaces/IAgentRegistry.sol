// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title IAgentRegistry
/// @notice Registry of agents — an agent is (owner, strategy, seal, label).
interface IAgentRegistry {
    enum Status {
        Active,
        Paused,
        Halted
    }

    struct Agent {
        bytes32 id;
        address owner;
        address strategy;
        bytes32 seal;
        uint64 createdAt;
        uint64 updatedAt;
        Status status;
        string name;
    }

    event AgentSummoned(
        address indexed owner,
        bytes32 indexed agentId,
        address strategy,
        bytes32 seal,
        string name
    );
    event AgentStatusChanged(bytes32 indexed agentId, Status from, Status to, string reason);
    event AgentSealRotated(bytes32 indexed agentId, bytes32 oldSeal, bytes32 newSeal);

    function summon(
        string calldata name,
        address strategy,
        bytes32 sealId
    ) external returns (bytes32 agentId);

    function pause(bytes32 agentId, string calldata reason) external;

    function resume(bytes32 agentId) external;

    function halt(bytes32 agentId, string calldata reason) external;

    function rotateSeal(bytes32 agentId, bytes32 newSeal) external;

    // ── Views ────────────────────────────────────────────────────────────────

    function agentOf(bytes32 agentId) external view returns (Agent memory);

    function agentsOf(address owner) external view returns (Agent[] memory);

    function idsOf(address owner) external view returns (bytes32[] memory);

    function isActive(bytes32 agentId) external view returns (bool);

    function exists(bytes32 agentId) external view returns (bool);
}

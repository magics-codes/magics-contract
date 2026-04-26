// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title IMagicsRouter
/// @notice The single entry point through which an agent acts on its owner's
///         balance. Holds an internal per-agent ledger, validates the seal,
///         and delegates execution to a strategy.
interface IMagicsRouter {
    event Deposited(bytes32 indexed agentId, address indexed token, uint256 amount);
    event Withdrawn(bytes32 indexed agentId, address indexed token, uint256 amount);
    event Cast(
        bytes32 indexed agentId,
        bytes32 indexed sealId,
        address indexed strategy,
        uint64 nonce,
        bytes32 dataHash
    );

    function deposit(bytes32 agentId, address token, uint256 amount) external;

    function withdraw(bytes32 agentId, address token, uint256 amount, address to) external;

    function cast(
        bytes32 agentId,
        uint64 deadline,
        bytes calldata data,
        bytes calldata signature
    ) external returns (bytes memory result);

    // ── Strategy-facing budget API (only callable inside an active cast) ─────

    function pull(address token, uint256 amount) external;

    function push(address token, address to, uint256 amount) external;

    function balanceOf(bytes32 agentId, address token) external view returns (uint256);
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {SealLib} from "../libraries/SealLib.sol";

/// @title ISealVault
/// @notice The vault that holds session-key boundaries for an owner. Mint,
///         revoke, and verify — those are the three motions.
interface ISealVault {
    event SealMinted(address indexed owner, bytes32 indexed sealId, SealLib.Seal seal);
    event SealRevoked(address indexed owner, bytes32 indexed sealId, string reason);
    event SealAllRevoked(address indexed owner, uint256 count);
    event SealConsumed(bytes32 indexed sealId, uint256 value, uint64 nonce);

    /// @notice Mint a new seal under msg.sender. Returns its deterministic id.
    function mint(SealLib.Seal calldata seal) external returns (bytes32 sealId);

    /// @notice Revoke a single seal. Only the owner of the seal may call.
    function revoke(bytes32 sealId, string calldata reason) external;

    /// @notice Revoke every active seal under msg.sender. Emergency motion.
    function revokeAll() external returns (uint256 revoked);

    /// @notice Verify a Cast signature against the seal pointed to by `sealId`.
    ///         Reverts with a specific error if any check fails. Returns the
    ///         current nonce after consumption.
    function verifyAndConsume(
        bytes32 sealId,
        bytes32 agentId,
        uint64 deadline,
        bytes32 dataHash,
        uint256 callValue,
        bytes calldata signature
    ) external returns (uint64 nonce);

    // ── Views ────────────────────────────────────────────────────────────────

    function ownerOf(bytes32 sealId) external view returns (address);

    function sealOf(bytes32 sealId) external view returns (SealLib.Seal memory);

    function nonceOf(bytes32 sealId) external view returns (uint64);

    function sealsOf(address owner) external view returns (bytes32[] memory);

    function isRevoked(bytes32 sealId) external view returns (bool);

    function isActive(bytes32 sealId) external view returns (bool);

    function dailyUsage(bytes32 sealId) external view returns (uint256 spent, uint64 windowStart);
}

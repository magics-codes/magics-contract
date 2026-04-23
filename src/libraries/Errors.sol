// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title Errors
/// @notice Centralised custom errors so revert reasons compile to 4-byte selectors
///         instead of long strings. Cheaper to deploy, easier to grep, and gives the
///         SDK a stable surface to decode against.
library Errors {
    // ── Access ────────────────────────────────────────────────────────────────
    error NotOwner(address caller);
    error NotSealVault(address caller);
    error NotRouter(address caller);
    error NotStrategy(address caller);

    // ── Seal lifecycle ────────────────────────────────────────────────────────
    error SealUnknown(bytes32 sealId);
    error SealExpired(bytes32 sealId, uint64 expiry);
    error SealRevoked(bytes32 sealId);
    error SealCapBreached(bytes32 sealId, uint256 attempted, uint256 cap);
    error SealSelectorMismatch(bytes32 sealId, bytes4 expected, bytes4 got);
    error SealTargetMismatch(bytes32 sealId, address expected, address got);
    error SealNonceReplay(bytes32 sealId, uint64 nonce);
    error SealSignatureInvalid(bytes32 sealId);

    // ── Agent lifecycle ───────────────────────────────────────────────────────
    error AgentUnknown(bytes32 agentId);
    error AgentNotActive(bytes32 agentId, uint8 status);
    error AgentNameEmpty();
    error AgentNameTooLong(uint256 length);
    error AgentStrategyZero();

    // ── Router ────────────────────────────────────────────────────────────────
    error CastDeadlinePassed(uint64 deadline);
    error CastBudgetExceeded(uint256 attempted, uint256 budget);
    error CastReentrant();
    error StrategyCallFailed(bytes returnData);
    error TokenZero();
    error AmountZero();
    error InsufficientBalance(uint256 have, uint256 want);

    // ── Generic ───────────────────────────────────────────────────────────────
    error ZeroAddress();
    error AlreadyInitialised();
}

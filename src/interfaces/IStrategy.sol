// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title IStrategy
/// @notice A strategy is the body of a cast. The router calls `execute` with
///         the agent context and arbitrary encoded data. The strategy decides
///         what that means.
///
/// @dev Strategies MUST be stateless w.r.t. caller funds — funds live in the
///      router's internal ledger, and the strategy asks the router to move
///      them via the budget it was granted for this cast.
interface IStrategy {
    /// @notice Human-readable identifier (e.g. "passive-yield/v1").
    function name() external view returns (string memory);

    /// @notice Semantic version of the strategy logic.
    function version() external view returns (string memory);

    /// @notice Hash committed to by the seal's `scopeHash`. Lets owners pin a
    ///         seal to a specific strategy revision.
    function scopeHash() external view returns (bytes32);

    /// @notice Execute one cast. Called by the router under a budgeted context.
    /// @param agentId    The agent this cast belongs to.
    /// @param owner      The agent's owner. Strategies should never trust this
    ///                   from anywhere but the router.
    /// @param data       Strategy-specific encoded arguments.
    /// @return result    Strategy-specific encoded result.
    function execute(
        bytes32 agentId,
        address owner,
        bytes calldata data
    ) external returns (bytes memory result);
}

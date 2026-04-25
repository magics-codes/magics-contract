// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title SealLib
/// @notice Pure helpers for the Seal struct: hashing, EIP-712 typing, scope checks.
///         Lives in a library so the same logic can be used inside the vault and
///         from off-chain code generated from the ABI.
library SealLib {
    /// @dev A seal is a written boundary around a session key. Whoever holds the
    ///      private key for `signer` can perform exactly the actions described by
    ///      the rest of the struct — nothing else, never past expiry.
    struct Seal {
        address signer;       // ephemeral key that signs cast intents
        address target;       // contract the signer may call (router, usually)
        bytes4 selector;      // function selector on `target`. bytes4(0) = wildcard
        uint128 valueCap;     // max ETH value per individual call
        uint128 dailyCap;     // max total ETH value within any 24h window
        uint64 expiry;        // unix seconds after which the seal is dead
        uint64 createdAt;     // unix seconds at mint
        bytes32 scopeHash;    // arbitrary opaque tag (e.g. strategy hash)
    }

    /// @dev EIP-712 typehash for the Cast action — what the seal signer is asked
    ///      to sign before the router will route a call.
    /// keccak256("Cast(bytes32 agentId,uint64 nonce,uint64 deadline,bytes32 dataHash)")
    /// TODO: replace placeholder with compile-time keccak256 once typehash is locked in.
    bytes32 internal constant CAST_TYPEHASH =
        0x9ea6b1a4c4f7c1f3a2c25f1d51b3b4d2e9e62f5cd2c1a4d8c4e8a3b8b7f2c2c1;

    /// @dev Type hash used to derive a stable on-chain id for a seal.
    /// keccak256("Seal(address signer,address target,bytes4 selector,uint128 valueCap,uint128 dailyCap,uint64 expiry,uint64 createdAt,bytes32 scopeHash)")
    /// TODO: same.
    bytes32 internal constant SEAL_TYPEHASH =
        0x1a3f4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b;

    /// @notice Deterministic id for a seal. Two seals with the same parameters
    ///         and owner collide — which is fine, since they are interchangeable.
    function idOf(address owner, Seal memory s) internal pure returns (bytes32) {
        return keccak256(
            abi.encode(
                SEAL_TYPEHASH,
                owner,
                s.signer,
                s.target,
                s.selector,
                s.valueCap,
                s.dailyCap,
                s.expiry,
                s.createdAt,
                s.scopeHash
            )
        );
    }

    /// @notice The struct hash that goes into the EIP-712 digest for a Cast.
    function castStructHash(
        bytes32 agentId,
        uint64 nonce,
        uint64 deadline,
        bytes32 dataHash
    ) internal pure returns (bytes32) {
        return keccak256(abi.encode(CAST_TYPEHASH, agentId, nonce, deadline, dataHash));
    }

    /// @notice True if `s` is currently within its lifetime window.
    function isLive(Seal memory s, uint256 nowTs) internal pure returns (bool) {
        return nowTs < s.expiry && nowTs >= s.createdAt;
    }

    /// @notice True if `selector` is permitted by `s`. Wildcard (bytes4(0)) matches
    ///         every selector; otherwise an exact match is required.
    function permitsSelector(Seal memory s, bytes4 selector) internal pure returns (bool) {
        return s.selector == bytes4(0) || s.selector == selector;
    }
}

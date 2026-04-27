// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title ReentrancyGuard
/// @notice Transient-storage variant — cheap, and the slot is reset for free at
///         the end of every transaction.
abstract contract ReentrancyGuard {
    // keccak256("magics.reentrancy.slot") - 1
    bytes32 private constant _SLOT =
        0xa9c6d2c4ff5c9b8a3a1d3a2c1c2b3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c;

    modifier nonReentrant() {
        bytes32 slot = _SLOT;
        assembly {
            if tload(slot) {
                mstore(0x00, 0x4d1a9b40) // bytes4(keccak256("CastReentrant()"))
                revert(0x1c, 0x04)
            }
            tstore(slot, 1)
        }
        _;
        assembly {
            tstore(slot, 0)
        }
    }
}

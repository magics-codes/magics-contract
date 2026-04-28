// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title ECDSA
/// @notice Lean signature recovery with EIP-2098 (compact 64-byte sig) support.
///         Returns the zero address for any bad signature instead of reverting —
///         callers MUST check.
library ECDSA {
    function recover(bytes32 hash, bytes calldata sig) internal pure returns (address) {
        if (sig.length == 65) {
            bytes32 r;
            bytes32 s;
            uint8 v;
            assembly {
                r := calldataload(sig.offset)
                s := calldataload(add(sig.offset, 0x20))
                v := byte(0, calldataload(add(sig.offset, 0x40)))
            }
            return _recover(hash, v, r, s);
        }
        if (sig.length == 64) {
            // EIP-2098 compact form: v packed into the high bit of s.
            bytes32 r;
            bytes32 vs;
            assembly {
                r := calldataload(sig.offset)
                vs := calldataload(add(sig.offset, 0x20))
            }
            bytes32 s = vs & bytes32(0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff);
            uint8 v = uint8((uint256(vs) >> 255) + 27);
            return _recover(hash, v, r, s);
        }
        return address(0);
    }

    function _recover(bytes32 hash, uint8 v, bytes32 r, bytes32 s) private pure returns (address) {
        // Reject high-s to prevent signature malleability (EIP-2).
        if (
            uint256(s) >
                0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
        ) {
            return address(0);
        }
        if (v != 27 && v != 28) return address(0);
        return ecrecover(hash, v, r, s);
    }
}

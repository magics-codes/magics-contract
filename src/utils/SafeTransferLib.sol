// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title SafeTransferLib
/// @notice ERC20 transfer helpers that tolerate non-standard tokens (the kind
///         that don't return a bool). Reverts with a tight selector on any
///         failure path.
library SafeTransferLib {
    error TransferFailed();
    error TransferFromFailed();
    error ApproveFailed();

    function safeTransfer(address token, address to, uint256 amount) internal {
        bool ok;
        assembly {
            let m := mload(0x40)
            mstore(m, 0xa9059cbb00000000000000000000000000000000000000000000000000000000)
            mstore(add(m, 0x04), to)
            mstore(add(m, 0x24), amount)
            ok :=
                and(
                    or(iszero(returndatasize()), and(gt(returndatasize(), 31), eq(mload(0x00), 1))),
                    call(gas(), token, 0, m, 0x44, 0x00, 0x20)
                )
        }
        if (!ok) revert TransferFailed();
    }

    function safeTransferFrom(address token, address from, address to, uint256 amount) internal {
        bool ok;
        assembly {
            let m := mload(0x40)
            mstore(m, 0x23b872dd00000000000000000000000000000000000000000000000000000000)
            mstore(add(m, 0x04), from)
            mstore(add(m, 0x24), to)
            mstore(add(m, 0x44), amount)
            ok :=
                and(
                    or(iszero(returndatasize()), and(gt(returndatasize(), 31), eq(mload(0x00), 1))),
                    call(gas(), token, 0, m, 0x64, 0x00, 0x20)
                )
        }
        if (!ok) revert TransferFromFailed();
    }

    function safeApprove(address token, address spender, uint256 amount) internal {
        bool ok;
        assembly {
            let m := mload(0x40)
            mstore(m, 0x095ea7b300000000000000000000000000000000000000000000000000000000)
            mstore(add(m, 0x04), spender)
            mstore(add(m, 0x24), amount)
            ok :=
                and(
                    or(iszero(returndatasize()), and(gt(returndatasize(), 31), eq(mload(0x00), 1))),
                    call(gas(), token, 0, m, 0x44, 0x00, 0x20)
                )
        }
        if (!ok) revert ApproveFailed();
    }
}

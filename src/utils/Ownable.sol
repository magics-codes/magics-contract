// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {Errors} from "../libraries/Errors.sol";

/// @title Ownable
/// @notice Two-step ownership transfer. Lean enough to keep an opinion about
///         what the SDK can rely on (pending owner is observable), strict
///         enough to avoid accidental hand-offs.
abstract contract Ownable {
    address public owner;
    address public pendingOwner;

    event OwnershipTransferStarted(address indexed previousOwner, address indexed newOwner);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    modifier onlyOwner() {
        if (msg.sender != owner) revert Errors.NotOwner(msg.sender);
        _;
    }

    constructor(address initialOwner) {
        if (initialOwner == address(0)) revert Errors.ZeroAddress();
        owner = initialOwner;
        emit OwnershipTransferred(address(0), initialOwner);
    }

    function transferOwnership(address newOwner) external onlyOwner {
        pendingOwner = newOwner;
        emit OwnershipTransferStarted(owner, newOwner);
    }

    function acceptOwnership() external {
        address pending = pendingOwner;
        if (msg.sender != pending) revert Errors.NotOwner(msg.sender);
        address previous = owner;
        owner = pending;
        delete pendingOwner;
        emit OwnershipTransferred(previous, pending);
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IERC20} from "../interfaces/IERC20.sol";
import {MockERC20} from "./MockERC20.sol";

/// @notice Toy ERC-4626-like vault. Shares are 1:1 minus a configurable yield
///         knob that the tests can advance to simulate accruing returns.
contract MockYieldVault is MockERC20 {
    IERC20 public immutable underlying;
    uint256 public yieldBps; // basis points applied at redeem time

    constructor(address underlying_)
        MockERC20("Mock Yield Share", "myShare", 18)
    {
        underlying = IERC20(underlying_);
    }

    function asset() external view returns (address) {
        return address(underlying);
    }

    function setYieldBps(uint256 bps) external {
        yieldBps = bps;
    }

    function deposit(uint256 assets, address receiver) external returns (uint256 shares) {
        underlying.transferFrom(msg.sender, address(this), assets);
        shares = assets;
        _mintShares(receiver, shares);
    }

    function redeem(uint256 shares, address receiver, address owner)
        external
        returns (uint256 assets)
    {
        require(balanceOf[owner] >= shares, "vault: shares");
        if (owner != msg.sender) {
            uint256 a = allowance[owner][msg.sender];
            require(a >= shares, "vault: allowance");
            if (a != type(uint256).max) {
                allowance[owner][msg.sender] = a - shares;
            }
        }
        balanceOf[owner] -= shares;
        totalSupply -= shares;

        assets = (shares * (10_000 + yieldBps)) / 10_000;
        // Mint extra underlying to simulate yield arriving in the vault.
        uint256 have = underlying.balanceOf(address(this));
        if (assets > have) {
            MockERC20(address(underlying)).mint(address(this), assets - have);
        }
        underlying.transfer(receiver, assets);
        emit Transfer(owner, address(0), shares);
    }

    function previewRedeem(uint256 shares) external view returns (uint256) {
        return (shares * (10_000 + yieldBps)) / 10_000;
    }

    function _mintShares(address to, uint256 amount) internal {
        totalSupply += amount;
        unchecked {
            balanceOf[to] += amount;
        }
        emit Transfer(address(0), to, amount);
    }
}

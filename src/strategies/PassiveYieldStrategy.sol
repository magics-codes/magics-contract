// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IStrategy} from "../interfaces/IStrategy.sol";
import {IMagicsRouter} from "../interfaces/IMagicsRouter.sol";
import {IERC20} from "../interfaces/IERC20.sol";
import {SafeTransferLib} from "../utils/SafeTransferLib.sol";

/// @notice Minimal ERC-4626-like yield vault interface used by the strategy.
interface IYieldVault {
    function asset() external view returns (address);
    function deposit(uint256 assets, address receiver) external returns (uint256 shares);
    function redeem(uint256 shares, address receiver, address owner)
        external
        returns (uint256 assets);
    function previewRedeem(uint256 shares) external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
}

/// @title PassiveYieldStrategy
/// @notice Compound idle ERC20 deposits into a yield vault, harvest back on
///         request. The simplest end-to-end example of the cast lifecycle.
///
///         Action encoding (`bytes data`):
///         - bytes1 op   — 0x01 = compound, 0x02 = harvest
///         - uint256 amt — token amount (op=0x01) or shares to redeem (op=0x02)
///                         0 means "all available"
contract PassiveYieldStrategy is IStrategy {
    using SafeTransferLib for address;

    bytes1 internal constant OP_COMPOUND = 0x01;
    bytes1 internal constant OP_HARVEST = 0x02;

    IMagicsRouter public immutable router;
    IYieldVault public immutable yieldVault;
    address public immutable asset;

    /// @dev Shares deposited on behalf of each agent. The strategy holds them
    ///      itself rather than the router, so a halt/migration is a single
    ///      redeem call.
    mapping(bytes32 => uint256) public sharesOf;

    error UnknownOp(bytes1 op);
    error NotRouter();
    error NothingToCompound();
    error NothingToHarvest();

    event Compounded(bytes32 indexed agentId, uint256 assets, uint256 shares);
    event Harvested(bytes32 indexed agentId, uint256 shares, uint256 assets);

    constructor(address router_, address yieldVault_) {
        router = IMagicsRouter(router_);
        yieldVault = IYieldVault(yieldVault_);
        asset = IYieldVault(yieldVault_).asset();
        // Pre-approve the vault — saves a transaction per compound.
        asset.safeApprove(yieldVault_, type(uint256).max);
    }

    /// @inheritdoc IStrategy
    function name() external pure returns (string memory) {
        return "passive-yield/v1";
    }

    /// @inheritdoc IStrategy
    function version() external pure returns (string memory) {
        return "0.1.0";
    }

    /// @inheritdoc IStrategy
    function scopeHash() external view returns (bytes32) {
        return keccak256(abi.encodePacked("passive-yield/v1", asset, address(yieldVault)));
    }

    /// @inheritdoc IStrategy
    function execute(bytes32 agentId, address owner, bytes calldata data)
        external
        returns (bytes memory result)
    {
        if (msg.sender != address(router)) revert NotRouter();
        if (data.length < 1) revert UnknownOp(0x00);

        bytes1 op = data[0];
        uint256 amount = data.length >= 33 ? _readUint256(data, 1) : 0;

        if (op == OP_COMPOUND) {
            (uint256 spent, uint256 shares) = _compound(agentId, amount);
            return abi.encode(spent, shares);
        }
        if (op == OP_HARVEST) {
            (uint256 burnt, uint256 received) = _harvest(agentId, amount);
            owner; // owner is informational here — funds re-credit to agent
            return abi.encode(burnt, received);
        }
        revert UnknownOp(op);
    }

    // ── Internals ─────────────────────────────────────────────────────────────

    function _compound(bytes32 agentId, uint256 requested)
        internal
        returns (uint256 spent, uint256 shares)
    {
        uint256 available = router.balanceOf(agentId, asset);
        spent = requested == 0 ? available : requested;
        if (spent == 0 || spent > available) revert NothingToCompound();

        router.pull(asset, spent);
        shares = yieldVault.deposit(spent, address(this));
        sharesOf[agentId] += shares;
        emit Compounded(agentId, spent, shares);
    }

    function _harvest(bytes32 agentId, uint256 requested)
        internal
        returns (uint256 burnt, uint256 received)
    {
        uint256 have = sharesOf[agentId];
        burnt = requested == 0 ? have : requested;
        if (burnt == 0 || burnt > have) revert NothingToHarvest();

        unchecked {
            sharesOf[agentId] = have - burnt;
        }
        received = yieldVault.redeem(burnt, address(this), address(this));
        // Push assets back into the agent's router balance.
        IERC20(asset).approve(address(router), received);
        router.push(asset, address(0), received);
        emit Harvested(agentId, burnt, received);
    }

    function _readUint256(bytes calldata data, uint256 offset) internal pure returns (uint256 v) {
        assembly {
            v := calldataload(add(data.offset, offset))
        }
    }
}

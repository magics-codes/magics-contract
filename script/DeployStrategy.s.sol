// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {Script, console2} from "forge-std/Script.sol";

import {PassiveYieldStrategy} from "../src/strategies/PassiveYieldStrategy.sol";

/// @title DeployStrategy
/// @notice Deploys the PassiveYieldStrategy against an already-deployed router
///         and an external yield vault address. Strategies are intentionally
///         out of the core deploy — protocol-as-substrate, strategies-as-add-on.
contract DeployStrategy is Script {
    function run() public returns (address strategy) {
        uint256 pk = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address router = vm.envAddress("ROUTER_ADDRESS");
        address yieldVault = vm.envAddress("YIELD_VAULT_ADDRESS");

        vm.startBroadcast(pk);
        PassiveYieldStrategy s = new PassiveYieldStrategy(router, yieldVault);
        vm.stopBroadcast();

        strategy = address(s);
        console2.log("PassiveYieldStrategy", strategy);
    }
}

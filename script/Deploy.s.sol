// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {Script, console2} from "forge-std/Script.sol";

import {SealVault} from "../src/SealVault.sol";
import {AgentRegistry} from "../src/AgentRegistry.sol";
import {MagicsRouter} from "../src/MagicsRouter.sol";

/// @title Deploy
/// @notice One transaction per contract. Uses the deployer's nonce to predict
///         the router address, which the vault locks in at construction.
///
///         Run:
///           forge script script/Deploy.s.sol \
///             --rpc-url base \
///             --broadcast \
///             --verify \
///             --etherscan-api-key $BASESCAN_API_KEY
contract Deploy is Script {
    struct Deployment {
        address sealVault;
        address agentRegistry;
        address magicsRouter;
        uint256 chainId;
        uint256 deployedAt;
    }

    function run() public returns (Deployment memory out) {
        uint256 pk = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(pk);
        uint64 nonce = vm.getNonce(deployer);

        // SealVault -> AgentRegistry -> MagicsRouter, so the router takes the
        // nonce+2 slot. `computeCreateAddress` is from forge-std/StdUtils.
        address predictedRouter = computeCreateAddress(deployer, nonce + 2);

        vm.startBroadcast(pk);
        SealVault vault = new SealVault(predictedRouter);
        AgentRegistry registry = new AgentRegistry(address(vault));
        MagicsRouter router = new MagicsRouter(address(vault), address(registry));
        vm.stopBroadcast();

        require(address(router) == predictedRouter, "router address drift");

        out = Deployment({
            sealVault: address(vault),
            agentRegistry: address(registry),
            magicsRouter: address(router),
            chainId: block.chainid,
            deployedAt: block.timestamp
        });

        console2.log("chain id           ", out.chainId);
        console2.log("SealVault          ", out.sealVault);
        console2.log("AgentRegistry      ", out.agentRegistry);
        console2.log("MagicsRouter       ", out.magicsRouter);

        _writeDeployment(out);
    }

    /// @dev Writes the address set to deployments/{chainId}.json. Picked up by
    ///      the CLI and the web app at runtime.
    function _writeDeployment(Deployment memory d) internal {
        string memory json = string.concat(
            "{\n",
            '  "chainId": ',
            vm.toString(d.chainId),
            ",\n",
            '  "deployedAt": ',
            vm.toString(d.deployedAt),
            ",\n",
            '  "contracts": {\n',
            '    "SealVault": "',
            vm.toString(d.sealVault),
            '",\n',
            '    "AgentRegistry": "',
            vm.toString(d.agentRegistry),
            '",\n',
            '    "MagicsRouter": "',
            vm.toString(d.magicsRouter),
            '"\n',
            "  }\n}\n"
        );
        string memory path =
            string.concat("deployments/", vm.toString(d.chainId), ".json");
        vm.writeFile(path, json);
    }
}

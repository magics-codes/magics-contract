// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {Script, console2} from "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";

/// @title Verify
/// @notice Reads deployments/{chainId}.json and prints the `forge verify-contract`
///         commands you can paste — handy when the in-deploy verify step misses
///         a contract or the explorer was rate-limited at the time.
contract Verify is Script {
    using stdJson for string;

    function run() public view {
        string memory path =
            string.concat("deployments/", vm.toString(block.chainid), ".json");
        string memory raw = vm.readFile(path);

        address sealVault = raw.readAddress(".contracts.SealVault");
        address agentRegistry = raw.readAddress(".contracts.AgentRegistry");
        address magicsRouter = raw.readAddress(".contracts.MagicsRouter");

        console2.log("# Paste these into your shell (basescan):");
        console2.log("");
        _printVerify("src/SealVault.sol:SealVault", sealVault, abi.encode(magicsRouter));
        _printVerify("src/AgentRegistry.sol:AgentRegistry", agentRegistry, abi.encode(sealVault));
        _printVerify(
            "src/MagicsRouter.sol:MagicsRouter",
            magicsRouter,
            abi.encode(sealVault, agentRegistry)
        );
    }

    function _printVerify(string memory contractName, address at, bytes memory ctor)
        internal
        view
    {
        console2.log(
            string.concat(
                "forge verify-contract ",
                vm.toString(at),
                " ",
                contractName,
                " --chain ",
                vm.toString(block.chainid),
                " --constructor-args ",
                _bytesToHex(ctor)
            )
        );
        console2.log("");
    }

    function _bytesToHex(bytes memory b) internal pure returns (string memory) {
        bytes memory alphabet = "0123456789abcdef";
        bytes memory out = new bytes(2 + b.length * 2);
        out[0] = "0";
        out[1] = "x";
        for (uint256 i; i < b.length; ++i) {
            out[2 + i * 2] = alphabet[uint8(b[i]) >> 4];
            out[3 + i * 2] = alphabet[uint8(b[i]) & 0x0f];
        }
        return string(out);
    }
}

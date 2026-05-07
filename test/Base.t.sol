// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {Test} from "forge-std/Test.sol";

import {SealVault} from "../src/SealVault.sol";
import {AgentRegistry} from "../src/AgentRegistry.sol";
import {MagicsRouter} from "../src/MagicsRouter.sol";
import {PassiveYieldStrategy} from "../src/strategies/PassiveYieldStrategy.sol";
import {MockERC20} from "../src/mocks/MockERC20.sol";
import {MockYieldVault} from "../src/mocks/MockYieldVault.sol";
import {SealLib} from "../src/libraries/SealLib.sol";

/// @dev Shared scaffolding. Each test contract extends this so the wiring
///      stays identical and individual files focus on intent.
abstract contract BaseTest is Test {
    SealVault internal vault;
    AgentRegistry internal registry;
    MagicsRouter internal router;
    PassiveYieldStrategy internal strategy;
    MockERC20 internal usdc;
    MockYieldVault internal yieldVault;

    address internal owner;
    uint256 internal signerKey;
    address internal signer;

    function setUp() public virtual {
        owner = makeAddr("owner");
        (signer, signerKey) = makeAddrAndKey("seal-signer");

        usdc = new MockERC20("USD Coin", "USDC", 6);
        yieldVault = new MockYieldVault(address(usdc));

        // The vault has to know the router and vice versa — predict router
        // address using CREATE nonce. The three deploys land at +0 (vault),
        // +1 (registry), +2 (router) relative to the current nonce.
        address routerPrediction = computeCreateAddress(address(this), vm.getNonce(address(this)) + 1);
        vault = new SealVault(routerPrediction);
        registry = new AgentRegistry(address(vault));
        router = new MagicsRouter(address(vault), address(registry));
        require(address(router) == routerPrediction, "router prediction off");

        strategy = new PassiveYieldStrategy(address(router), address(yieldVault));

        usdc.mint(owner, 1_000_000e6);
        vm.prank(owner);
        usdc.approve(address(router), type(uint256).max);
    }

    function _mintSeal(uint128 dailyCap, uint64 expiry) internal returns (bytes32 sealId) {
        SealLib.Seal memory s = SealLib.Seal({
            signer: signer,
            target: address(router),
            selector: MagicsRouter.cast.selector,
            valueCap: 0,
            dailyCap: dailyCap,
            expiry: expiry,
            createdAt: 0, // overwritten by the vault
            scopeHash: strategy.scopeHash()
        });
        vm.prank(owner);
        sealId = vault.mint(s);
    }

    function _summon(bytes32 sealId, string memory name) internal returns (bytes32 agentId) {
        vm.prank(owner);
        agentId = registry.summon(name, address(strategy), sealId);
    }

    function _signCast(
        bytes32 agentId,
        uint64 deadline,
        bytes memory data,
        uint64 nonce
    ) internal view returns (bytes memory sig) {
        bytes32 dataHash = keccak256(data);
        bytes32 structHash = SealLib.castStructHash(agentId, nonce, deadline, dataHash);
        bytes32 digest = keccak256(
            abi.encodePacked("\x19\x01", vault.domainSeparator(), structHash)
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, digest);
        sig = abi.encodePacked(r, s, v);
    }
}

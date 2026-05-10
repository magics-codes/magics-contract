// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaseTest} from "./Base.t.sol";

contract PassiveYieldStrategyTest is BaseTest {
    function test_compound_then_harvest_returnsYield() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 agentId = _summon(sealId, "yield-cycle");

        vm.prank(owner);
        router.deposit(agentId, address(usdc), 10_000e6);

        // Compound — op 0x01, amount 0 = all.
        bytes memory data = abi.encodePacked(bytes1(0x01), uint256(0));
        uint64 deadline = uint64(block.timestamp + 60);
        bytes memory sig = _signCast(agentId, deadline, data, 0);
        router.cast(agentId, deadline, data, sig);

        assertEq(router.balanceOf(agentId, address(usdc)), 0, "all pulled");
        assertEq(strategy.sharesOf(agentId), 10_000e6, "shares minted 1:1");

        // Advance virtual yield (+5%).
        yieldVault.setYieldBps(500);

        // Harvest — op 0x02, amount 0 = all shares.
        bytes memory data2 = abi.encodePacked(bytes1(0x02), uint256(0));
        bytes memory sig2 = _signCast(agentId, deadline, data2, 1);
        router.cast(agentId, deadline, data2, sig2);

        assertEq(strategy.sharesOf(agentId), 0, "shares burnt");
        assertEq(router.balanceOf(agentId, address(usdc)), 10_500e6, "+5% yield credited");
    }

    function test_compound_replayNonceFails() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 agentId = _summon(sealId, "replayer");
        vm.prank(owner);
        router.deposit(agentId, address(usdc), 1_000e6);

        bytes memory data = abi.encodePacked(bytes1(0x01), uint256(100e6));
        uint64 deadline = uint64(block.timestamp + 60);
        bytes memory sig = _signCast(agentId, deadline, data, 0);

        router.cast(agentId, deadline, data, sig);

        // Same signature, same nonce — the second time the vault expects nonce
        // 1 in the digest, so signature recovery diverges → SealSignatureInvalid.
        vm.expectRevert();
        router.cast(agentId, deadline, data, sig);
    }
}

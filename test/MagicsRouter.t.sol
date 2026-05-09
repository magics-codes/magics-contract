// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaseTest} from "./Base.t.sol";
import {Errors} from "../src/libraries/Errors.sol";
import {IAgentRegistry} from "../src/interfaces/IAgentRegistry.sol";

contract MagicsRouterTest is BaseTest {
    function test_deposit_credits() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 agentId = _summon(sealId, "depositor");

        vm.prank(owner);
        router.deposit(agentId, address(usdc), 1_000e6);

        assertEq(router.balanceOf(agentId, address(usdc)), 1_000e6);
        assertEq(usdc.balanceOf(address(router)), 1_000e6);
    }

    function test_withdraw_debits() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 agentId = _summon(sealId, "depositor");

        vm.prank(owner);
        router.deposit(agentId, address(usdc), 1_000e6);
        vm.prank(owner);
        router.withdraw(agentId, address(usdc), 400e6, owner);

        assertEq(router.balanceOf(agentId, address(usdc)), 600e6);
        assertEq(usdc.balanceOf(owner), 1_000_000e6 - 600e6);
    }

    function test_withdraw_rejectsNonOwner() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 agentId = _summon(sealId, "vic");
        vm.prank(owner);
        router.deposit(agentId, address(usdc), 1_000e6);

        address attacker = makeAddr("attacker");
        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSelector(Errors.NotOwner.selector, attacker));
        router.withdraw(agentId, address(usdc), 100e6, attacker);
    }

    function test_cast_haltedAgentReverts() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 agentId = _summon(sealId, "halt-me");
        vm.prank(owner);
        router.deposit(agentId, address(usdc), 1_000e6);

        vm.prank(owner);
        registry.halt(agentId, "drift");

        uint64 deadline = uint64(block.timestamp + 60);
        bytes memory data = abi.encodePacked(bytes1(0x01), uint256(0));
        bytes memory sig = _signCast(agentId, deadline, data, 0);

        vm.expectRevert(
            abi.encodeWithSelector(
                Errors.AgentNotActive.selector,
                agentId,
                uint8(IAgentRegistry.Status.Halted)
            )
        );
        router.cast(agentId, deadline, data, sig);
    }

    function test_cast_deadlinePassedReverts() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 agentId = _summon(sealId, "tardy");
        uint64 deadline = uint64(block.timestamp + 10);
        bytes memory data = abi.encodePacked(bytes1(0x01), uint256(0));
        bytes memory sig = _signCast(agentId, deadline, data, 0);

        vm.warp(deadline + 1);
        vm.expectRevert(abi.encodeWithSelector(Errors.CastDeadlinePassed.selector, deadline));
        router.cast(agentId, deadline, data, sig);
    }
}

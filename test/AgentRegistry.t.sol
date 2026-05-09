// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaseTest} from "./Base.t.sol";
import {IAgentRegistry} from "../src/interfaces/IAgentRegistry.sol";
import {Errors} from "../src/libraries/Errors.sol";

contract AgentRegistryTest is BaseTest {
    function test_summon_storesAgent() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 7 days));
        bytes32 agentId = _summon(sealId, "yield-bot");

        IAgentRegistry.Agent memory a = registry.agentOf(agentId);
        assertEq(a.owner, owner);
        assertEq(a.strategy, address(strategy));
        assertEq(a.seal, sealId);
        assertEq(uint8(a.status), uint8(IAgentRegistry.Status.Active));
        assertEq(a.name, "yield-bot");
    }

    function test_summon_rejectsForeignSeal() public {
        // Mint a seal owned by `owner`, attempt summon from someone else.
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        address rando = makeAddr("rando");
        vm.prank(rando);
        vm.expectRevert(abi.encodeWithSelector(Errors.NotOwner.selector, rando));
        registry.summon("hijack", address(strategy), sealId);
    }

    function test_pauseAndResume_changesStatus() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 agentId = _summon(sealId, "pauser");

        vm.prank(owner);
        registry.pause(agentId, "rebalancing");
        assertEq(uint8(registry.agentOf(agentId).status), uint8(IAgentRegistry.Status.Paused));

        vm.prank(owner);
        registry.resume(agentId);
        assertEq(uint8(registry.agentOf(agentId).status), uint8(IAgentRegistry.Status.Active));
    }

    function test_halt_isTerminal() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 agentId = _summon(sealId, "soon-dead");

        vm.prank(owner);
        registry.halt(agentId, "drift");

        // Resume from halted should revert.
        vm.prank(owner);
        vm.expectRevert();
        registry.resume(agentId);
    }

    function test_agentsOf_returnsAll() public {
        bytes32 s1 = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        bytes32 a1 = _summon(s1, "one");

        // mint a different seal by altering dailyCap; SealVault dedup keys on
        // the seal struct so a different cap = different id.
        bytes32 s2 = _mintSeal(2 ether, uint64(block.timestamp + 1 days));
        bytes32 a2 = _summon(s2, "two");

        IAgentRegistry.Agent[] memory agents = registry.agentsOf(owner);
        assertEq(agents.length, 2, "two agents");
        assertTrue(agents[0].id == a1 || agents[1].id == a1);
        assertTrue(agents[0].id == a2 || agents[1].id == a2);
    }
}

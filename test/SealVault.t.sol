// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaseTest} from "./Base.t.sol";
import {SealLib} from "../src/libraries/SealLib.sol";
import {Errors} from "../src/libraries/Errors.sol";

contract SealVaultTest is BaseTest {
    function test_mint_emitsAndStores() public {
        uint64 expiry = uint64(block.timestamp + 1 days);
        bytes32 sealId = _mintSeal(1 ether, expiry);

        SealLib.Seal memory s = vault.sealOf(sealId);
        assertEq(s.signer, signer, "signer");
        assertEq(s.target, address(router), "target");
        assertEq(s.expiry, expiry, "expiry");

        assertEq(vault.ownerOf(sealId), owner, "ownerOf");
        assertTrue(vault.isActive(sealId), "active");
        assertEq(vault.nonceOf(sealId), 0, "nonce starts at 0");
    }

    function test_mint_rejectsExpiryInPast() public {
        SealLib.Seal memory s = SealLib.Seal({
            signer: signer,
            target: address(router),
            selector: bytes4(0),
            valueCap: 0,
            dailyCap: 1 ether,
            expiry: uint64(block.timestamp - 1),
            createdAt: 0,
            scopeHash: bytes32(0)
        });
        vm.prank(owner);
        vm.expectRevert();
        vault.mint(s);
    }

    function test_revoke_onlyOwner() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        address attacker = makeAddr("attacker");

        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSelector(Errors.NotOwner.selector, attacker));
        vault.revoke(sealId, "nope");

        vm.prank(owner);
        vault.revoke(sealId, "rotating");
        assertTrue(vault.isRevoked(sealId), "revoked");
        assertFalse(vault.isActive(sealId), "not active");
        assertEq(vault.revokeReasonOf(sealId), "rotating", "reason recorded");
    }

    function test_revokeAll_revokesEveryLive() public {
        _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        // Make a second distinct seal by varying dailyCap.
        SealLib.Seal memory s2 = SealLib.Seal({
            signer: signer,
            target: address(router),
            selector: bytes4(0),
            valueCap: 0,
            dailyCap: 2 ether,
            expiry: uint64(block.timestamp + 1 days),
            createdAt: 0,
            scopeHash: bytes32(0)
        });
        vm.prank(owner);
        vault.mint(s2);

        vm.prank(owner);
        uint256 n = vault.revokeAll();
        assertEq(n, 2, "two revoked");

        bytes32[] memory ids = vault.sealsOf(owner);
        for (uint256 i; i < ids.length; ++i) {
            assertTrue(vault.isRevoked(ids[i]), "seal revoked");
        }
    }

    function test_verifyAndConsume_onlyRouter() public {
        bytes32 sealId = _mintSeal(1 ether, uint64(block.timestamp + 1 days));
        vm.expectRevert(abi.encodeWithSelector(Errors.NotRouter.selector, address(this)));
        vault.verifyAndConsume(sealId, bytes32(0), uint64(block.timestamp + 60), bytes32(0), 0, "");
    }
}

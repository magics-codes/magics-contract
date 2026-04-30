// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {ISealVault} from "./interfaces/ISealVault.sol";
import {SealLib} from "./libraries/SealLib.sol";
import {Errors} from "./libraries/Errors.sol";
import {EIP712} from "./utils/EIP712.sol";
import {ECDSA} from "./utils/ECDSA.sol";

/// @title SealVault
/// @notice The boundary store. Every seal is a written limit on an ephemeral key:
///         which contract it may call, which function on that contract, how
///         much ETH value per call, how much per day, and until when. Verifying
///         a Cast routes through here — and so does revocation.
///
/// @dev    Bound to a single MagicsRouter at construction. Verification mutates
///         (nonce + daily window), so we only let one trusted address advance
///         state. Other code can still read.
contract SealVault is ISealVault, EIP712 {
    using SealLib for SealLib.Seal;

    /// @dev Each seal pairs the parameters with mutable usage counters.
    struct SealRecord {
        SealLib.Seal seal;
        address owner;
        uint64 nonce;          // next valid nonce
        uint64 windowStart;    // unix seconds — start of current 24h window
        uint128 windowSpent;   // ETH value spent inside the current window
        bool revoked;
        string revokeReason;   // free-form, gas-cheap because rarely set
    }

    /// @dev The single address allowed to advance seal state (consume nonce,
    ///      bump daily window). Anyone can still read.
    address public immutable router;

    mapping(bytes32 => SealRecord) internal _records;
    mapping(address => bytes32[]) internal _sealsByOwner;

    uint256 private constant _WINDOW = 1 days;

    constructor(address router_) EIP712("magics", "1") {
        if (router_ == address(0)) revert Errors.ZeroAddress();
        router = router_;
    }

    // ── Owner-facing motions ──────────────────────────────────────────────────

    /// @inheritdoc ISealVault
    function mint(SealLib.Seal calldata sealIn) external returns (bytes32 sealId) {
        if (sealIn.signer == address(0) || sealIn.target == address(0)) {
            revert Errors.ZeroAddress();
        }
        if (sealIn.expiry <= block.timestamp) revert Errors.SealExpired(bytes32(0), sealIn.expiry);

        SealLib.Seal memory s = sealIn;
        s.createdAt = uint64(block.timestamp);

        sealId = SealLib.idOf(msg.sender, s);
        // If a record already exists under this id, it must belong to msg.sender
        // and be revoked — we'll re-issue with a fresh nonce/window.
        SealRecord storage r = _records[sealId];
        if (r.owner != address(0)) {
            if (r.owner != msg.sender) revert Errors.SealUnknown(sealId);
            if (!r.revoked) revert Errors.AlreadyInitialised();
        } else {
            _sealsByOwner[msg.sender].push(sealId);
        }
        r.seal = s;
        r.owner = msg.sender;
        r.nonce = 0;
        r.windowStart = uint64(block.timestamp);
        r.windowSpent = 0;
        r.revoked = false;
        delete r.revokeReason;

        emit SealMinted(msg.sender, sealId, s);
    }

    /// @inheritdoc ISealVault
    function revoke(bytes32 sealId, string calldata reason) external {
        SealRecord storage r = _records[sealId];
        if (r.owner != msg.sender) revert Errors.NotOwner(msg.sender);
        if (r.revoked) revert Errors.SealRevoked(sealId);
        r.revoked = true;
        r.revokeReason = reason;
        emit SealRevoked(msg.sender, sealId, reason);
    }

    /// @inheritdoc ISealVault
    function revokeAll() external returns (uint256 revoked) {
        bytes32[] storage ids = _sealsByOwner[msg.sender];
        uint256 n = ids.length;
        for (uint256 i; i < n; ++i) {
            SealRecord storage r = _records[ids[i]];
            if (!r.revoked) {
                r.revoked = true;
                r.revokeReason = "revoke-all";
                unchecked {
                    ++revoked;
                }
            }
        }
        emit SealAllRevoked(msg.sender, revoked);
    }

    // ── Router-facing motion ──────────────────────────────────────────────────

    /// @inheritdoc ISealVault
    function verifyAndConsume(
        bytes32 sealId,
        bytes32 agentId,
        uint64 deadline,
        bytes32 dataHash,
        uint256 callValue,
        bytes calldata signature
    ) external returns (uint64 nonce) {
        if (msg.sender != router) revert Errors.NotRouter(msg.sender);

        SealRecord storage r = _records[sealId];
        if (r.owner == address(0)) revert Errors.SealUnknown(sealId);
        if (r.revoked) revert Errors.SealRevoked(sealId);

        SealLib.Seal memory s = r.seal;
        if (block.timestamp >= s.expiry) revert Errors.SealExpired(sealId, s.expiry);
        if (deadline < block.timestamp) revert Errors.CastDeadlinePassed(deadline);

        if (callValue > s.valueCap) {
            revert Errors.SealCapBreached(sealId, callValue, s.valueCap);
        }

        // Roll the daily window if needed, then enforce dailyCap.
        uint64 windowStart = r.windowStart;
        uint128 windowSpent = r.windowSpent;
        if (block.timestamp >= windowStart + _WINDOW) {
            windowStart = uint64(block.timestamp);
            windowSpent = 0;
        }
        uint256 projected = uint256(windowSpent) + callValue;
        if (projected > s.dailyCap) {
            revert Errors.SealCapBreached(sealId, projected, s.dailyCap);
        }

        bytes32 structHash = SealLib.castStructHash(agentId, r.nonce, deadline, dataHash);
        bytes32 digest = _hashTypedDataV4(structHash);
        address recovered = ECDSA.recover(digest, signature);
        if (recovered == address(0) || recovered != s.signer) {
            revert Errors.SealSignatureInvalid(sealId);
        }

        unchecked {
            nonce = r.nonce;
            r.nonce = nonce + 1;
        }
        r.windowStart = windowStart;
        r.windowSpent = uint128(projected);

        emit SealConsumed(sealId, callValue, nonce);
    }

    // ── Views ─────────────────────────────────────────────────────────────────

    /// @inheritdoc ISealVault
    function ownerOf(bytes32 sealId) external view returns (address) {
        return _records[sealId].owner;
    }

    /// @inheritdoc ISealVault
    function sealOf(bytes32 sealId) external view returns (SealLib.Seal memory) {
        return _records[sealId].seal;
    }

    /// @inheritdoc ISealVault
    function nonceOf(bytes32 sealId) external view returns (uint64) {
        return _records[sealId].nonce;
    }

    /// @inheritdoc ISealVault
    function sealsOf(address owner) external view returns (bytes32[] memory) {
        return _sealsByOwner[owner];
    }

    /// @inheritdoc ISealVault
    function isRevoked(bytes32 sealId) external view returns (bool) {
        return _records[sealId].revoked;
    }

    /// @inheritdoc ISealVault
    function isActive(bytes32 sealId) external view returns (bool) {
        SealRecord storage r = _records[sealId];
        if (r.owner == address(0) || r.revoked) return false;
        return block.timestamp < r.seal.expiry;
    }

    /// @inheritdoc ISealVault
    function dailyUsage(bytes32 sealId)
        external
        view
        returns (uint256 spent, uint64 windowStart)
    {
        SealRecord storage r = _records[sealId];
        if (block.timestamp >= uint256(r.windowStart) + _WINDOW) {
            return (0, uint64(block.timestamp));
        }
        return (r.windowSpent, r.windowStart);
    }

    function revokeReasonOf(bytes32 sealId) external view returns (string memory) {
        return _records[sealId].revokeReason;
    }
}

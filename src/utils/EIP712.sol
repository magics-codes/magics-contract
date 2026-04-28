// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title EIP712
/// @notice Minimal EIP-712 domain separator with chain-id invalidation. Reads
///         the cached separator if the chain id hasn't moved, otherwise
///         rebuilds — so a fork can't replay signatures across chains.
abstract contract EIP712 {
    bytes32 private constant _TYPE_HASH =
        keccak256(
            "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
        );

    bytes32 private immutable _CACHED_DOMAIN_SEPARATOR;
    uint256 private immutable _CACHED_CHAIN_ID;
    address private immutable _CACHED_THIS;

    bytes32 private immutable _HASHED_NAME;
    bytes32 private immutable _HASHED_VERSION;

    constructor(string memory domainName, string memory domainVersion) {
        bytes32 hashedName = keccak256(bytes(domainName));
        bytes32 hashedVersion = keccak256(bytes(domainVersion));

        _HASHED_NAME = hashedName;
        _HASHED_VERSION = hashedVersion;

        _CACHED_CHAIN_ID = block.chainid;
        _CACHED_THIS = address(this);
        _CACHED_DOMAIN_SEPARATOR = _buildDomainSeparator(hashedName, hashedVersion);
    }

    function _domainSeparatorV4() internal view returns (bytes32) {
        if (address(this) == _CACHED_THIS && block.chainid == _CACHED_CHAIN_ID) {
            return _CACHED_DOMAIN_SEPARATOR;
        }
        return _buildDomainSeparator(_HASHED_NAME, _HASHED_VERSION);
    }

    function _buildDomainSeparator(bytes32 nameHash, bytes32 versionHash)
        private
        view
        returns (bytes32)
    {
        return keccak256(
            abi.encode(_TYPE_HASH, nameHash, versionHash, block.chainid, address(this))
        );
    }

    function _hashTypedDataV4(bytes32 structHash) internal view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", _domainSeparatorV4(), structHash));
    }

    function domainSeparator() external view returns (bytes32) {
        return _domainSeparatorV4();
    }
}

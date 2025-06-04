use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::Hasher;

use crate::claim_tree_constants;

/// Represents the data that is hashed to form a leaf in the Merkle tree.
/// Each leaf corresponds to a unique claimant's entitlement within a specific cohort.
///
/// ## ⚠️ CRITICAL: Borsh Serialization Schema Stability
///
/// **This struct's Borsh serialization format MUST NEVER CHANGE** after mainnet deployment.
/// Any modification to field order, field types, or addition/removal of fields will:
/// - Break all existing merkle trees
/// - Invalidate all generated proofs
/// - Require coordinated protocol upgrade across all deployments
///
/// ### Current Schema (Version 1.0 - IMMUTABLE):
/// ```
/// use anchor_lang::prelude::Pubkey;
///
/// struct ClaimLeaf {
///     claimant: Pubkey,              // 32 bytes, offset 0
///     assigned_vault_index: u8,      // 1 byte,  offset 32
///     entitlements: u64,             // 8 bytes, offset 33
/// }
/// // Total: 41 bytes
/// ```
///
/// ### Forbidden Changes:
/// - Reordering fields
/// - Changing field types (u8 -> u16, u64 -> u32, etc.)
/// - Adding new fields (even optional ones)
/// - Removing existing fields
/// - Changing from/to Option<T> types
///
/// ### If Schema Changes Are Required:
/// 1. Create a new ClaimLeafV2 struct
/// 2. Update tree builders to use new version
/// 3. Keep old version for backward compatibility
/// 4. Coordinate upgrade across all deployments
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct ClaimLeaf {
    /// The public key of the recipient.
    pub claimant: Pubkey,
    /// The index of the vault assigned for this claim (0-based).
    /// The actual vault pubkey is derived as a PDA from the cohort address and this index.
    pub assigned_vault_index: u8,
    /// The number of entitlements (e.g., number of NFTs held, specific tier count)
    /// for which the claimant is eligible for the reward_per_entitlement.
    pub entitlements: u64,
}

impl ClaimLeaf {
    /// Hash this ClaimLeaf to produce a 32-byte hash suitable for Merkle tree construction.
    /// This follows our merkle tree hashing scheme: SHA256(0x00 || borsh_serialized_leaf_data).
    pub fn to_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::default();

        // Prepend 0x00 for a leaf node, following common Solana merkle tree patterns
        hasher.hash(&[claim_tree_constants::LEAF_PREFIX]);

        // Serialize the leaf data using Borsh
        let serialized_leaf = self.try_to_vec().expect("Failed to serialize ClaimLeaf");

        // Hash the serialized data
        hasher.hash(&serialized_leaf);

        hasher.result().to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_claim_leaf_consistent() {
        let key1 = Pubkey::new_unique();
        let leaf1_v1 = ClaimLeaf {
            claimant: key1,
            assigned_vault_index: 0,
            entitlements: 10,
        };
        let leaf1_v2 = ClaimLeaf {
            claimant: key1,
            assigned_vault_index: 0,
            entitlements: 10,
        };

        let key2 = Pubkey::new_unique();
        let leaf2 = ClaimLeaf {
            claimant: key2,
            assigned_vault_index: 0,
            entitlements: 5,
        };

        let hash1_v1 = leaf1_v1.to_hash();
        let hash1_v2 = leaf1_v2.to_hash();
        let hash2 = leaf2.to_hash();

        assert_eq!(
            hash1_v1, hash1_v2,
            "Hashes for identical leaves should be the same."
        );
        assert_ne!(
            hash1_v1, hash2,
            "Hashes for different leaves should be different."
        );
    }

    #[test]
    fn test_hash_claim_leaf_prefix() {
        // Test that the 0x00 prefix influences the hash.
        // Manually hash with and without the prefix to compare.
        let leaf = ClaimLeaf {
            claimant: Pubkey::new_unique(),
            assigned_vault_index: 0,
            entitlements: 1,
        };
        let serialized_leaf = leaf.try_to_vec().unwrap();

        // Hash with 0x00 prefix (as done by to_hash)
        let prefixed_hash_bytes = leaf.to_hash();

        // Manual hash without 0x00 prefix
        let mut direct_hasher = Hasher::default();
        direct_hasher.hash(&serialized_leaf);
        let direct_hash_bytes = direct_hasher.result().to_bytes();

        assert_ne!(
            prefixed_hash_bytes, direct_hash_bytes,
            "Prefixed hash (with 0x00 prefix) should differ from direct hash of serialized data."
        );

        // Manual hash with 0x00 prefix to confirm logic
        let mut manual_prefixed_hasher = Hasher::default();
        manual_prefixed_hasher.hash(&[claim_tree_constants::LEAF_PREFIX]);
        manual_prefixed_hasher.hash(&serialized_leaf);
        let manual_prefixed_hash_bytes = manual_prefixed_hasher.result().to_bytes();

        assert_eq!(
            prefixed_hash_bytes, manual_prefixed_hash_bytes,
            "to_hash should match manual prefixed hashing."
        );
    }

    // =================================================================
    // 🔒 BORSH SCHEMA STABILITY TESTS
    // =================================================================
    // These tests protect against accidental schema changes that would
    // break all existing merkle trees and proofs.

    #[test]
    fn test_borsh_schema_size_stability() {
        // CRITICAL: ClaimLeaf must always serialize to exactly 41 bytes
        // Pubkey(32) + u8(1) + u64(8) = 41 bytes
        let leaf = ClaimLeaf {
            claimant: Pubkey::new_from_array([1u8; 32]),
            assigned_vault_index: 255,
            entitlements: u64::MAX,
        };

        let serialized = leaf.try_to_vec().unwrap();
        assert_eq!(
            serialized.len(),
            41,
            "❌ SCHEMA BREAKING CHANGE: ClaimLeaf serialization size changed from 41 bytes to {}. This will break all existing merkle trees!",
            serialized.len()
        );
    }

    #[test]
    fn test_borsh_schema_field_order_stability() {
        // CRITICAL: Field order must never change
        let test_pubkey = Pubkey::new_from_array([42u8; 32]);
        let leaf = ClaimLeaf {
            claimant: test_pubkey,
            assigned_vault_index: 123,
            entitlements: 456789,
        };

        let serialized = leaf.try_to_vec().unwrap();

        // Verify field layout:
        // Bytes 0-31: claimant (Pubkey)
        assert_eq!(
            &serialized[0..32],
            test_pubkey.as_ref(),
            "❌ SCHEMA BREAKING CHANGE: claimant field moved from bytes 0-31"
        );

        // Byte 32: assigned_vault_index (u8)
        assert_eq!(
            serialized[32], 123,
            "❌ SCHEMA BREAKING CHANGE: assigned_vault_index field moved from byte 32"
        );

        // Bytes 33-40: entitlements (u64, little-endian)
        let entitlements_bytes = &serialized[33..41];
        let deserialized_entitlements = u64::from_le_bytes(entitlements_bytes.try_into().unwrap());
        assert_eq!(
            deserialized_entitlements,
            456789,
            "❌ SCHEMA BREAKING CHANGE: entitlements field moved from bytes 33-40 or endianness changed"
        );
    }

    #[test]
    fn test_borsh_schema_round_trip_stability() {
        // Test that serialization and deserialization are perfectly stable
        let original_leaf = ClaimLeaf {
            claimant: Pubkey::new_from_array([0x5Au8; 32]),
            assigned_vault_index: 42,
            entitlements: 0xDEADBEEFCAFEBABE,
        };

        let serialized = original_leaf.try_to_vec().unwrap();
        let deserialized = ClaimLeaf::try_from_slice(&serialized).unwrap();

        assert_eq!(
            original_leaf, deserialized,
            "❌ SCHEMA BREAKING CHANGE: ClaimLeaf round-trip serialization failed"
        );
    }

    #[test]
    fn test_borsh_schema_specific_byte_layout() {
        // Test the exact byte layout to catch any subtle changes
        let leaf = ClaimLeaf {
            claimant: Pubkey::new_from_array([0u8; 32]), // All zeros
            assigned_vault_index: 1,
            entitlements: 256, // 0x0100 in little-endian
        };

        let serialized = leaf.try_to_vec().unwrap();
        let expected = {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&[0u8; 32]); // claimant: 32 zero bytes
            bytes.push(1); // assigned_vault_index: 1 byte
            bytes.extend_from_slice(&256u64.to_le_bytes()); // entitlements: 8 bytes little-endian
            bytes
        };

        assert_eq!(
            serialized, expected,
            "❌ SCHEMA BREAKING CHANGE: ClaimLeaf byte layout doesn't match expected format"
        );
    }

    #[test]
    fn test_known_hash_stability() {
        // CRITICAL: Test against known hash values to detect any changes
        // If this test fails, it means the hash function or schema changed
        let fixed_leaf = ClaimLeaf {
            claimant: Pubkey::new_from_array([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
                0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
                0x1d, 0x1e, 0x1f, 0x20,
            ]),
            assigned_vault_index: 42,
            entitlements: 1337,
        };

        let computed_hash = fixed_leaf.to_hash();

        // This hash was computed with the current implementation
        // If it changes, something in the schema or hash function changed
        let expected_hash: [u8; 32] = [
            0xbd, 0x28, 0x41, 0x89, 0x21, 0x74, 0xd4, 0xf3, 0x75, 0xf5, 0x09, 0x7e, 0xa7, 0x4a,
            0x4c, 0x95, 0x5d, 0x61, 0xa8, 0x39, 0xcc, 0xe9, 0xf5, 0xfa, 0x7d, 0x3d, 0x25, 0xf3,
            0x89, 0xc9, 0xea, 0x2b,
        ];

        assert_eq!(
            computed_hash,
            expected_hash,
            "❌ HASH BREAKING CHANGE: ClaimLeaf hash changed! This will invalidate all existing proofs.\nExpected: {:?}\nActual: {:?}",
            expected_hash,
            computed_hash
        );
    }

    #[test]
    fn test_edge_case_values_stability() {
        // Test serialization with edge case values to ensure stability
        let edge_cases = [
            // Minimum values
            ClaimLeaf {
                claimant: Pubkey::new_from_array([0u8; 32]),
                assigned_vault_index: 0,
                entitlements: 0,
            },
            // Maximum values
            ClaimLeaf {
                claimant: Pubkey::new_from_array([0xFFu8; 32]),
                assigned_vault_index: u8::MAX,
                entitlements: u64::MAX,
            },
        ];

        for (i, leaf) in edge_cases.iter().enumerate() {
            let serialized = leaf.try_to_vec().unwrap();
            assert_eq!(
                serialized.len(),
                41,
                "❌ SCHEMA BREAKING CHANGE: Edge case {} has wrong serialization size",
                i
            );

            let deserialized = ClaimLeaf::try_from_slice(&serialized).unwrap();
            assert_eq!(
                *leaf, deserialized,
                "❌ SCHEMA BREAKING CHANGE: Edge case {} failed round-trip",
                i
            );
        }
    }
}

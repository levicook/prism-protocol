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
///     campaign: Pubkey,     // 32 bytes, offset 0
///     claimant: Pubkey,     // 32 bytes, offset 32
///     entitlements: u64,    //  8 bytes, offset 65
///     vault_index: u8,      //  1 byte,  offset 64
/// }
/// // Total: 73 bytes (32 + 32 + 8 + 1)
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
    /// The public key of the campaign.
    pub campaign: Pubkey,
    /// The public key of the recipient.
    pub claimant: Pubkey,
    /// The number of entitlements (e.g., number of NFTs held, specific tier count)
    /// for which the claimant is eligible for the reward_per_entitlement.
    pub entitlements: u64,
    /// The index of the vault assigned for this claim (0-based).
    /// The actual vault pubkey is derived as a PDA from the cohort address and this index.
    pub vault_index: u8,
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
        let key2 = Pubkey::new_unique();
        let key3 = Pubkey::new_unique();

        let leaf1_v1 = ClaimLeaf {
            campaign: key1,
            claimant: key2,
            entitlements: 10,
            vault_index: 0,
        };
        let leaf1_v2 = ClaimLeaf {
            campaign: key1,
            claimant: key2,
            entitlements: 10,
            vault_index: 0,
        };

        let leaf2_v1 = ClaimLeaf {
            campaign: key1,
            claimant: key3,
            entitlements: 5,
            vault_index: 0,
        };

        let hash1_v1 = leaf1_v1.to_hash();
        let hash1_v2 = leaf1_v2.to_hash();
        let hash2_v1 = leaf2_v1.to_hash();

        assert_eq!(
            hash1_v1, hash1_v2,
            "Hashes for identical leaves should be the same."
        );
        assert_ne!(
            hash1_v1, hash2_v1,
            "Hashes for different leaves should be different."
        );
    }

    #[test]
    fn test_hash_claim_leaf_prefix() {
        // Test that the 0x00 prefix influences the hash.
        // Manually hash with and without the prefix to compare.
        let leaf = ClaimLeaf {
            campaign: Pubkey::new_unique(),
            claimant: Pubkey::new_unique(),
            entitlements: 1,
            vault_index: 0,
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
        // CRITICAL: ClaimLeaf must always serialize to exactly 73 bytes
        // Pubkey(32) + Pubkey(32) + u64(8) + u8(1) = 73 bytes
        let leaf = ClaimLeaf {
            campaign: Pubkey::new_from_array([1u8; 32]),
            claimant: Pubkey::new_from_array([1u8; 32]),
            entitlements: u64::MAX,
            vault_index: 255,
        };

        let serialized = leaf.try_to_vec().unwrap();
        assert_eq!(
            serialized.len(),
            73,
            "❌ SCHEMA BREAKING CHANGE: ClaimLeaf serialization size changed from 73 bytes to {}. This will break all existing merkle trees!",
            serialized.len()
        );
    }

    #[test]
    fn test_borsh_schema_field_order_stability() {
        // CRITICAL: Field order must never change
        let test_campaign = Pubkey::new_from_array([41u8; 32]);
        let test_claimant = Pubkey::new_from_array([42u8; 32]);
        let leaf = ClaimLeaf {
            campaign: test_campaign,
            claimant: test_claimant,
            entitlements: 456789,
            vault_index: 123,
        };

        let serialized = leaf.try_to_vec().unwrap();

        // Verify field layout:
        // Bytes 0-31: campaign (Pubkey)
        assert_eq!(
            &serialized[0..32],
            test_campaign.as_ref(),
            "❌ SCHEMA BREAKING CHANGE: campaign field moved from bytes 0-31"
        );

        // Bytes 32-63: claimant (Pubkey)
        assert_eq!(
            &serialized[32..64],
            test_claimant.as_ref(),
            "❌ SCHEMA BREAKING CHANGE: claimant field moved from bytes 32-63"
        );

        // Bytes 64-71: entitlements (u64, little-endian)
        let entitlements_bytes = &serialized[64..72];
        let deserialized_entitlements = u64::from_le_bytes(entitlements_bytes.try_into().unwrap());
        assert_eq!(
            deserialized_entitlements,
            456789,
            "❌ SCHEMA BREAKING CHANGE: entitlements field moved from bytes 64-71 or endianness changed"
        );

        // Byte 72: vault_index (u8)
        assert_eq!(
            serialized[72], 123,
            "❌ SCHEMA BREAKING CHANGE: vault_index field moved from byte 72"
        );
    }

    #[test]
    fn test_borsh_schema_round_trip_stability() {
        // Test that serialization and deserialization are perfectly stable
        let original_leaf = ClaimLeaf {
            campaign: Pubkey::new_from_array([0x5Au8; 32]),
            claimant: Pubkey::new_from_array([0x5Bu8; 32]),
            entitlements: 0xDEADBEEFCAFEBABE,
            vault_index: 42,
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
            campaign: Pubkey::new_from_array([0u8; 32]), // All zeros
            claimant: Pubkey::new_from_array([1u8; 32]), // All ones
            entitlements: 256,                           // 0x0100 in little-endian
            vault_index: 1,
        };

        let serialized = leaf.try_to_vec().unwrap();
        let expected = {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&[0u8; 32]); // campaign: 32 zero bytes
            bytes.extend_from_slice(&[1u8; 32]); // claimant: 32 one bytes
            bytes.extend_from_slice(&256u64.to_le_bytes()); // entitlements: 8 bytes little-endian
            bytes.push(1); // vault_index: 1 byte
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
            campaign: Pubkey::new_from_array([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
                0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
                0x1d, 0x1e, 0x1f, 0x20,
            ]),
            claimant: Pubkey::new_from_array([
                0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
                0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c,
                0x3d, 0x3e, 0x3f, 0x40,
            ]),
            entitlements: 1337,
            vault_index: 42,
        };

        let computed_hash = fixed_leaf.to_hash();

        // This hash was computed with the new schema (v2.0) including campaign field
        // If it changes, something in the schema or hash function changed
        let expected_hash: [u8; 32] = [
            163, 25, 170, 103, 140, 31, 91, 127, 227, 225, 226, 216, 246, 25, 19, 128, 114, 133,
            171, 254, 49, 204, 24, 27, 231, 116, 207, 136, 237, 75, 62, 106,
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
                campaign: Pubkey::new_from_array([0u8; 32]),
                claimant: Pubkey::new_from_array([0u8; 32]),
                entitlements: 0,
                vault_index: 0,
            },
            // Maximum values
            ClaimLeaf {
                campaign: Pubkey::new_from_array([0xFFu8; 32]),
                claimant: Pubkey::new_from_array([0xFFu8; 32]),
                entitlements: u64::MAX,
                vault_index: u8::MAX,
            },
        ];

        for (i, leaf) in edge_cases.iter().enumerate() {
            let serialized = leaf.try_to_vec().unwrap();
            assert_eq!(
                serialized.len(),
                73,
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

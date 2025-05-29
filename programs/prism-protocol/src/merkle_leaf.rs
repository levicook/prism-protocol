use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::Hasher;

/// Represents the data that is hashed to form a leaf in the Merkle tree.
/// Each leaf corresponds to a unique claimant's entitlement within a specific cohort.
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

/// Hashes a `ClaimLeaf` to produce a 32-byte hash suitable for Merkle tree construction.
/// This follows our merkle tree hashing scheme: SHA256(0x00 || borsh_serialized_leaf_data).
pub fn hash_claim_leaf(leaf_data: &ClaimLeaf) -> [u8; 32] {
    let mut hasher = Hasher::default();

    // Prepend 0x00 for a leaf node, following common Solana merkle tree patterns
    hasher.hash(&[0x00]);

    // Serialize the leaf data using Borsh
    let serialized_leaf = leaf_data
        .try_to_vec()
        .expect("Failed to serialize ClaimLeaf");

    // Hash the serialized data
    hasher.hash(&serialized_leaf);

    hasher.result().to_bytes()
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

        let hash1_v1 = hash_claim_leaf(&leaf1_v1);
        let hash1_v2 = hash_claim_leaf(&leaf1_v2);
        let hash2 = hash_claim_leaf(&leaf2);

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

        // Hash with 0x00 prefix (as done by hash_claim_leaf)
        let prefixed_hash_bytes = hash_claim_leaf(&leaf);

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
        manual_prefixed_hasher.hash(&[0x00]);
        manual_prefixed_hasher.hash(&serialized_leaf);
        let manual_prefixed_hash_bytes = manual_prefixed_hasher.result().to_bytes();

        assert_eq!(
            prefixed_hash_bytes, manual_prefixed_hash_bytes,
            "hash_claim_leaf should match manual prefixed hashing."
        );
    }
}

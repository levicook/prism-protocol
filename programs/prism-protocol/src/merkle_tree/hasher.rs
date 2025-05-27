use anchor_lang::solana_program::hash::Hasher as SolanaHasher;
use rs_merkle::Hasher;

/// SPL-compatible hasher that implements the same hashing logic as used in
/// the claim_tokens verification. This ensures that merkle trees built with
/// this hasher will produce proofs that can be verified on-chain.
///
/// Hashing scheme:
/// - Leaf nodes: SHA256(0x00 || leaf_data)
/// - Internal nodes: SHA256(0x01 || left_hash || right_hash)
/// - Child hashes are ordered lexicographically before concatenation
#[derive(Clone)]
pub struct SplHasher;

impl Hasher for SplHasher {
    type Hash = [u8; 32];

    fn hash(data: &[u8]) -> [u8; 32] {
        // This is used for leaf hashing
        let mut hasher = SolanaHasher::default();
        hasher.hash(&[0x00]); // Leaf prefix
        hasher.hash(data);
        hasher.result().to_bytes()
    }

    fn concat_and_hash(left: &Self::Hash, right: Option<&Self::Hash>) -> Self::Hash {
        match right {
            Some(right_hash) => {
                // This is used for internal node hashing
                let mut hasher = SolanaHasher::default();
                hasher.hash(&[0x01]); // Internal node prefix

                // Order hashes lexicographically (same as in verify_spl_merkle_proof)
                if left.as_ref() <= right_hash.as_ref() {
                    hasher.hash(left);
                    hasher.hash(right_hash);
                } else {
                    hasher.hash(right_hash);
                    hasher.hash(left);
                }

                hasher.result().to_bytes()
            }
            None => {
                // If no right sibling, just propagate the left hash
                // This matches the default behavior
                *left
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merkle_leaf::{hash_claim_leaf, ClaimLeaf};
    use anchor_lang::{prelude::Pubkey, AnchorSerialize as _};

    #[test]
    fn test_spl_hasher_leaf_compatibility() {
        // Test that our hasher produces the same result as hash_claim_leaf
        let claimant = Pubkey::new_unique();
        let assigned_vault = Pubkey::new_unique();
        let entitlements = 42;

        let leaf = ClaimLeaf {
            claimant,
            assigned_vault,
            entitlements,
        };

        // Hash using the existing hash_claim_leaf function
        let expected_hash = hash_claim_leaf(&leaf);

        // Hash using our SplHasher
        let leaf_data = leaf.try_to_vec().expect("Failed to serialize leaf");
        let actual_hash = SplHasher::hash(&leaf_data);

        assert_eq!(
            expected_hash, actual_hash,
            "SplHasher should produce the same hash as hash_claim_leaf"
        );
    }

    #[test]
    fn test_spl_hasher_internal_node_ordering() {
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];

        // Test both orderings produce the same result
        let result1 = SplHasher::concat_and_hash(&hash1, Some(&hash2));
        let result2 = SplHasher::concat_and_hash(&hash2, Some(&hash1));

        assert_eq!(
            result1, result2,
            "Hash ordering should be consistent regardless of input order"
        );

        // Verify the result matches manual calculation
        let mut expected_hasher = SolanaHasher::default();
        expected_hasher.hash(&[0x01]); // Internal node prefix
        expected_hasher.hash(&hash1); // hash1 < hash2 lexicographically
        expected_hasher.hash(&hash2);
        let expected = expected_hasher.result().to_bytes();

        assert_eq!(result1, expected, "Hash should match manual calculation");
    }
}

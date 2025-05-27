use anchor_lang::solana_program::hash::Hasher as SolanaHasher;
use rs_merkle::Hasher;

/// Merkle tree hasher for the Prism Protocol that implements the same hashing logic
/// as used in the claim_tokens verification. This ensures that merkle trees built with
/// this hasher will produce proofs that can be verified on-chain.
///
/// ## Security: Domain Separation via Prefixes
///
/// This implementation uses prefix bytes to provide **domain separation** between
/// leaf nodes and internal nodes, which is critical for preventing second preimage
/// attacks (also known as leaf-node confusion attacks).
///
/// **The Problem Without Prefixes:**
/// Without domain separation, an attacker could potentially:
/// - Take the hash of an internal node (concatenation of two child hashes)
/// - Present it as if it were a leaf node containing that same data
/// - Forge proofs by exploiting hash collisions between different node types
///
/// **The Solution:**
/// By prefixing each hash with a type identifier, we ensure that:
/// - Leaf hashes can never equal internal node hashes (different prefixes)
/// - Each hash has clear semantic meaning about what data produced it
/// - Proof forgery becomes cryptographically infeasible
///
/// ## Hashing Scheme
///
/// This follows standard cryptographic merkle tree practices:
/// - **Leaf nodes**: `SHA256(0x00 || leaf_data)` - The 0x00 prefix identifies this as leaf data
/// - **Internal nodes**: `SHA256(0x01 || left_hash || right_hash)` - The 0x01 prefix identifies this as an internal node
/// - **Child ordering**: Hashes are ordered lexicographically before concatenation for deterministic tree structure
///
/// This approach is used by Bitcoin, Ethereum, Certificate Transparency, and other
/// security-critical merkle tree implementations.
#[derive(Clone)]
pub struct PrismHasher;

impl Hasher for PrismHasher {
    type Hash = [u8; 32];

    fn hash(data: &[u8]) -> [u8; 32] {
        // This is used for leaf hashing
        let mut hasher = SolanaHasher::default();
        hasher.hash(&[0x00]); // Leaf prefix - prevents leaf/internal node confusion attacks
        hasher.hash(data);
        hasher.result().to_bytes()
    }

    fn concat_and_hash(left: &Self::Hash, right: Option<&Self::Hash>) -> Self::Hash {
        match right {
            Some(right_hash) => {
                // This is used for internal node hashing
                let mut hasher = SolanaHasher::default();
                hasher.hash(&[0x01]); // Internal node prefix - provides domain separation from leaf nodes

                // Order hashes lexicographically (same as in verify_merkle_proof)
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
    fn test_prism_hasher_leaf_consistency() {
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

        // Hash using our PrismHasher
        let leaf_data = leaf.try_to_vec().expect("Failed to serialize leaf");
        let actual_hash = PrismHasher::hash(&leaf_data);

        assert_eq!(
            expected_hash, actual_hash,
            "PrismHasher should produce the same hash as hash_claim_leaf"
        );
    }

    #[test]
    fn test_prism_hasher_internal_node_ordering() {
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];

        // Test both orderings produce the same result
        let result1 = PrismHasher::concat_and_hash(&hash1, Some(&hash2));
        let result2 = PrismHasher::concat_and_hash(&hash2, Some(&hash1));

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

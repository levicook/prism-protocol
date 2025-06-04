use anchor_lang::solana_program::hash::Hasher as SolanaHasher;
use anchor_lang::prelude::*;

/// Merkle tree hasher for 256-ary claim trees (V1) that implements the same domain
/// separation scheme as ClaimHasherV0 but supports up to 256 children per internal node.
///
/// ## Security: Domain Separation via Prefixes
///
/// This implementation uses the same prefix-based domain separation as V0:
/// - **Leaf nodes**: `SHA256(0x00 || leaf_data)` - Leaf prefix prevents confusion attacks
/// - **Internal nodes**: `SHA256(0x01 || sorted_child_hashes...)` - Internal node prefix
/// - **Child ordering**: All child hashes are sorted lexicographically for deterministic results
///
/// ## 256-ary Tree Structure
///
/// Unlike binary trees (V0) that have exactly 2 children per internal node, 256-ary trees (V1)
/// can have up to 256 children per internal node. This:
/// - Reduces tree depth significantly (log₂₅₆(n) vs log₂(n))
/// - Results in smaller proofs (fewer levels)
/// - Maintains the same security properties through domain separation
///
/// ## Implementation Notes
///
/// Since we can't use the `rs_merkle` crate (which only supports binary trees), this hasher
/// is designed to work with custom 256-ary tree building logic that will handle the tree
/// structure while using this hasher for the actual hash computations.
#[derive(Clone, Debug)]
pub struct ClaimHasherV1;

impl ClaimHasherV1 {
    /// Hash a leaf node using domain separation prefix.
    /// This produces the same result as `ClaimLeaf::to_hash()`.
    pub fn hash_leaf(data: &[u8]) -> [u8; 32] {
        let mut hasher = SolanaHasher::default();
        hasher.hash(&[0x00]); // Leaf prefix - prevents leaf/internal node confusion attacks
        hasher.hash(data);
        hasher.result().to_bytes()
    }

    /// Hash an internal node with up to 256 children.
    /// Children are sorted lexicographically before hashing for deterministic results.
    pub fn hash_internal_node(children: &[[u8; 32]]) -> [u8; 32] {
        assert!(!children.is_empty(), "Cannot hash empty children");
        assert!(children.len() <= 256, "Too many children for internal node (max 256)");

        // Sort children for deterministic ordering
        let mut sorted_children = children.to_vec();
        sorted_children.sort();

        let mut hasher = SolanaHasher::default();
        hasher.hash(&[0x01]); // Internal node prefix - provides domain separation from leaf nodes

        for child_hash in sorted_children {
            hasher.hash(&child_hash);
        }

        hasher.result().to_bytes()
    }

    /// Build a complete 256-ary merkle tree from leaf hashes.
    /// Returns the root hash and all tree levels for proof generation.
    pub fn build_tree(leaf_hashes: Vec<[u8; 32]>) -> Result<([u8; 32], Vec<Vec<[u8; 32]>>)> {
        if leaf_hashes.is_empty() {
            return Err(ErrorCode::InvalidInput.into());
        }

        if leaf_hashes.len() == 1 {
            // Single leaf case - the leaf hash is the root
            return Ok((leaf_hashes[0], vec![leaf_hashes]));
        }

        let mut current_level = leaf_hashes;
        let mut all_levels = vec![current_level.clone()];

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            // Process in chunks of up to 256 (256-ary tree)
            for chunk in current_level.chunks(256) {
                let parent_hash = Self::hash_internal_node(chunk);
                next_level.push(parent_hash);
            }

            all_levels.push(next_level.clone());
            current_level = next_level;
        }

        let root = current_level
            .first()
            .copied()
            .ok_or(ErrorCode::TreeBuildingFailed)?;

        Ok((root, all_levels))
    }

    /// Generate a 256-ary merkle proof for a specific leaf index.
    /// Returns the proof as Vec<Vec<[u8; 32]>> where each inner Vec contains
    /// the sibling hashes at that level.
    pub fn generate_proof(
        tree_levels: &[Vec<[u8; 32]>],
        leaf_index: usize,
    ) -> Result<Vec<Vec<[u8; 32]>>> {
        if tree_levels.is_empty() {
            return Err(ErrorCode::InvalidInput.into());
        }
        if leaf_index >= tree_levels[0].len() {
            return Err(ErrorCode::InvalidLeafIndex.into());
        }

        let mut proof = Vec::new();
        let mut current_index = leaf_index;

        // For each level (except the root level)
        for level in tree_levels.iter().take(tree_levels.len() - 1) {
            let mut siblings = Vec::new();

            // Calculate which "group of 256" this index belongs to
            let group_start = (current_index / 256) * 256;
            let group_end = std::cmp::min(group_start + 256, level.len());

            // Collect all siblings in this group (exclude the current index)
            for i in group_start..group_end {
                if i != current_index {
                    siblings.push(level[i]);
                }
            }

            proof.push(siblings);

            // Move to the next level - parent index is current_index / 256
            current_index /= 256;
        }

        Ok(proof)
    }

    /// Verify a 256-ary merkle proof.
    /// This is the same logic as in `ClaimProofV1::verify()` but as a standalone function.
    pub fn verify_proof(
        proof: &[Vec<[u8; 32]>],
        root: &[u8; 32],
        leaf_hash: &[u8; 32],
    ) -> bool {
        let mut computed_hash = *leaf_hash;

        // Process each level of the proof from bottom to top
        for level_siblings in proof.iter() {
            // Collect all hashes at this level: current computed hash + all siblings
            let mut level_hashes = Vec::with_capacity(level_siblings.len() + 1);
            level_hashes.push(computed_hash);
            level_hashes.extend_from_slice(level_siblings);

            // Sort all hashes lexicographically for deterministic ordering
            level_hashes.sort();

            // Hash the sorted children to get the parent node hash
            computed_hash = Self::hash_internal_node(&level_hashes);
        }

        computed_hash == *root
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Input cannot be empty")]
    InvalidInput,
    #[msg("Too many children for a single internal node (max 256)")]
    TooManyChildren,
    #[msg("Failed to build tree")]
    TreeBuildingFailed,
    #[msg("Invalid leaf index")]
    InvalidLeafIndex,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClaimLeaf;

    fn create_test_leaf(claimant_seed: u8, entitlements: u64) -> ClaimLeaf {
        // Create deterministic pubkey for testing
        let claimant = Pubkey::new_from_array([claimant_seed; 32]);
        ClaimLeaf {
            claimant,
            assigned_vault_index: 0,
            entitlements,
        }
    }

    #[test]
    fn test_leaf_hashing_compatibility() {
        // Test that ClaimHasherV1::hash_leaf produces the same result as ClaimLeaf::to_hash
        let leaf = create_test_leaf(42, 100);
        
        let expected_hash = leaf.to_hash();
        let leaf_data = leaf.try_to_vec().expect("Failed to serialize leaf");
        let actual_hash = ClaimHasherV1::hash_leaf(&leaf_data);

        assert_eq!(
            expected_hash, actual_hash,
            "ClaimHasherV1::hash_leaf should produce the same result as ClaimLeaf::to_hash"
        );
    }

    #[test]
    fn test_internal_node_hashing() {
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];
        let hash3 = [3u8; 32];

        // Test that ordering doesn't matter (hashes are sorted internally)
        let result1 = ClaimHasherV1::hash_internal_node(&[hash1, hash2, hash3]);
        let result2 = ClaimHasherV1::hash_internal_node(&[hash3, hash1, hash2]);
        let result3 = ClaimHasherV1::hash_internal_node(&[hash2, hash3, hash1]);

        assert_eq!(result1, result2, "Hash should be order-independent");
        assert_eq!(result2, result3, "Hash should be order-independent");

        // Test single child
        let single_result = ClaimHasherV1::hash_internal_node(&[hash1]);
        assert_ne!(single_result, hash1, "Internal node hash should differ from child hash");

        // Verify the result matches manual calculation
        let mut expected_hasher = SolanaHasher::default();
        expected_hasher.hash(&[0x01]); // Internal node prefix
        expected_hasher.hash(&hash1); // hash1 < hash2 < hash3 lexicographically
        expected_hasher.hash(&hash2);
        expected_hasher.hash(&hash3);
        let expected = expected_hasher.result().to_bytes();

        assert_eq!(result1, expected, "Hash should match manual calculation");
    }

    #[test]
    fn test_build_tree_single_leaf() {
        let leaf = create_test_leaf(1, 100);
        let leaf_hash = leaf.to_hash();

        let (root, levels) = ClaimHasherV1::build_tree(vec![leaf_hash]).unwrap();

        assert_eq!(root, leaf_hash, "Single leaf tree root should be the leaf hash");
        assert_eq!(levels.len(), 1, "Single leaf tree should have one level");
        assert_eq!(levels[0], vec![leaf_hash], "Level should contain the leaf hash");
    }

    #[test]
    fn test_build_tree_multiple_leaves() {
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
            create_test_leaf(4, 400),
        ];
        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|leaf| leaf.to_hash()).collect();

        let (root, levels) = ClaimHasherV1::build_tree(leaf_hashes.clone()).unwrap();

        // Should have 2 levels: leaf level + root level
        assert_eq!(levels.len(), 2, "Should have 2 levels for multiple leaves");
        assert_eq!(levels[0], leaf_hashes, "First level should be leaf hashes");
        assert_eq!(levels[1].len(), 1, "Root level should have one hash");
        assert_eq!(levels[1][0], root, "Root level should contain the root hash");

        // Verify root is computed correctly
        let expected_root = ClaimHasherV1::hash_internal_node(&leaf_hashes);
        assert_eq!(root, expected_root, "Root should match manual calculation");
    }

    #[test]
    fn test_generate_and_verify_proof() {
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
        ];
        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|leaf| leaf.to_hash()).collect();

        let (root, levels) = ClaimHasherV1::build_tree(leaf_hashes.clone()).unwrap();

        // Generate proof for first leaf (index 0)
        let proof = ClaimHasherV1::generate_proof(&levels, 0).unwrap();

        // Verify the proof
        let is_valid = ClaimHasherV1::verify_proof(&proof, &root, &leaf_hashes[0]);
        assert!(is_valid, "Proof should be valid");

        // Verify proof fails for wrong leaf
        let is_valid = ClaimHasherV1::verify_proof(&proof, &root, &leaf_hashes[1]);
        assert!(!is_valid, "Proof should be invalid for wrong leaf");

        // Verify proof fails for wrong root
        let wrong_root = [99u8; 32];
        let is_valid = ClaimHasherV1::verify_proof(&proof, &wrong_root, &leaf_hashes[0]);
        assert!(!is_valid, "Proof should be invalid for wrong root");
    }

    #[test]
    fn test_large_tree_proof_generation() {
        // Test with more leaves to ensure chunking works correctly
        let leaves: Vec<ClaimLeaf> = (0..10)
            .map(|i| create_test_leaf(i as u8, (i + 1) as u64 * 100))
            .collect();
        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|leaf| leaf.to_hash()).collect();

        let (root, levels) = ClaimHasherV1::build_tree(leaf_hashes.clone()).unwrap();

        // Generate and verify proofs for all leaves
        for (i, leaf_hash) in leaf_hashes.iter().enumerate() {
            let proof = ClaimHasherV1::generate_proof(&levels, i).unwrap();
            let is_valid = ClaimHasherV1::verify_proof(&proof, &root, leaf_hash);
            assert!(is_valid, "Proof for leaf {} should be valid", i);
        }
    }

    #[test]
    fn test_proof_compatibility_with_claim_proof_v1() {
        // Test that our hasher produces proofs compatible with ClaimProofV1::verify
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
        ];
        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|leaf| leaf.to_hash()).collect();

        let (root, levels) = ClaimHasherV1::build_tree(leaf_hashes.clone()).unwrap();
        let proof = ClaimHasherV1::generate_proof(&levels, 0).unwrap();

        // Create ClaimProofV1 and verify using its method
        let claim_proof = crate::ClaimProofV1::new(proof);
        let is_valid = claim_proof.verify(&root, &leaves[0]);
        assert!(is_valid, "ClaimProofV1 should be able to verify our proof");
    }

    #[test]
    fn test_error_conditions() {
        // Test empty input
        let result = ClaimHasherV1::build_tree(vec![]);
        assert!(result.is_err(), "Should error on empty input");

        // Test too many children for internal node - this should panic with assert!
        // We can't easily test this with assert! unless we use should_panic
    }

    #[test]
    #[should_panic(expected = "Too many children for internal node (max 256)")]
    fn test_too_many_children_panic() {
        let too_many_children = vec![[0u8; 32]; 257];
        let _result = ClaimHasherV1::hash_internal_node(&too_many_children);
    }
}

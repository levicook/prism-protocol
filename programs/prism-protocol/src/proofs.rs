use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::Hasher as SolanaHasher;

use crate::{claim_tree_constants, ClaimLeaf};

/// Unified proof type that can hold either binary (V0) or 256-ary (V1) merkle proofs.
/// This enables code reuse between claim_tokens_v0 and claim_tokens_v1 handlers.
#[derive(Clone, Debug)]
pub enum ClaimProof {
    /// Binary merkle tree proof (V0)
    V0(ClaimProofV0),
    /// 256-ary merkle tree proof (V1)
    V1(ClaimProofV1),
}

impl ClaimProof {
    /// Create a ProofType from a binary tree proof
    pub fn from_binary(proof: Vec<[u8; 32]>) -> Self {
        Self::V0(ClaimProofV0::new(proof))
    }

    /// Create a ProofType from a 256-ary tree proof
    pub fn from_wide(proof: Vec<Vec<[u8; 32]>>) -> Self {
        Self::V1(ClaimProofV1::new(proof))
    }

    /// Verify the proof against a root and leaf, regardless of proof type
    pub fn verify(&self, root: &[u8; 32], leaf: &ClaimLeaf) -> bool {
        match self {
            ClaimProof::V0(proof) => proof.verify(root, leaf),
            ClaimProof::V1(proof) => proof.verify(root, leaf),
        }
    }

    /// Get a descriptive name for logging
    pub fn description(&self) -> &'static str {
        match self {
            ClaimProof::V0(_) => "Binary merkle proof",
            ClaimProof::V1(_) => "256-ary merkle proof",
        }
    }

    /// Get the proof version for metrics/logging
    pub fn version(&self) -> u8 {
        match self {
            ClaimProof::V0(_) => 0,
            ClaimProof::V1(_) => 1,
        }
    }
}

/// Binary merkle tree proof for V0 claim instructions.
///
/// This wraps a Vec<[u8; 32]> to provide type safety and ensure
/// binary tree proofs can't be accidentally used with 256-ary tree verification.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct ClaimProofV0(pub Vec<[u8; 32]>);

impl ClaimProofV0 {
    /// Create a new binary tree proof
    pub fn new(proof: Vec<[u8; 32]>) -> Self {
        Self(proof)
    }

    /// Get proof elements as slice
    pub fn as_slice(&self) -> &[[u8; 32]] {
        &self.0
    }

    /// Get number of proof elements
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if proof is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consume wrapper and return inner Vec
    pub fn into_inner(self) -> Vec<[u8; 32]> {
        self.0
    }

    /// Verify a binary merkle tree proof using SHA256 hashing with domain separation.
    ///
    /// ## Security: Domain Separation
    /// This function enforces prefix-based domain separation to prevent second preimage attacks:
    /// - Leaf nodes are hashed as: SHA256(0x00 || borsh_serialized_leaf_data)
    /// - Internal nodes are hashed as: SHA256(0x01 || H(LeftChild) || H(RightChild))
    /// - Child hashes are ordered lexicographically before concatenation.
    ///
    /// The prefix bytes ensure that leaf hashes can never equal internal node hashes,
    /// preventing attackers from forging proofs by substituting node types.
    pub fn verify(&self, root: &[u8; 32], leaf: &ClaimLeaf) -> bool {
        let leaf_hash = leaf.to_hash();
        let mut computed_hash = leaf_hash;

        for p_elem in self.0.iter() {
            let mut hasher = SolanaHasher::default(); // Uses SHA256 by default
            hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]); // Internal node prefix - provides domain separation from leaf nodes (0x00)

            // Correctly order H(L) and H(R) before hashing for the parent node.
            if computed_hash <= *p_elem {
                hasher.hash(&computed_hash);
                hasher.hash(p_elem);
            } else {
                hasher.hash(p_elem);
                hasher.hash(&computed_hash);
            }
            computed_hash = hasher.result().to_bytes();
        }

        computed_hash == *root
    }
}

/// 256-ary merkle tree proof for V1 claim instructions.
///
/// This contains multiple levels of proof hashes, where each level can contain
/// up to 255 sibling hashes (since we use 256-ary trees). This structure provides
/// type safety and ensures 256-ary proofs can't be mixed with binary tree verification.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct ClaimProofV1(pub Vec<Vec<[u8; 32]>>);

impl ClaimProofV1 {
    /// Create a new 256-ary tree proof
    pub fn new(proof: Vec<Vec<[u8; 32]>>) -> Self {
        Self(proof)
    }

    /// Get proof levels as slice
    pub fn as_slice(&self) -> &[Vec<[u8; 32]>] {
        &self.0
    }

    /// Get number of levels in the proof
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if proof is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consume wrapper and return inner Vec<Vec<[u8; 32]>>
    pub fn into_inner(self) -> Vec<Vec<[u8; 32]>> {
        self.0
    }

    /// Get total number of hashes across all levels
    /// Useful for compute budget estimation
    pub fn total_hashes(&self) -> usize {
        self.0.iter().map(|level| level.len()).sum()
    }

    /// Get maximum level width (largest number of sibling hashes at any level)
    /// Useful for memory allocation planning
    pub fn max_level_width(&self) -> usize {
        self.0.iter().map(|level| level.len()).max().unwrap_or(0)
    }

    /// Verify a 256-ary merkle tree proof using SHA256 hashing with domain separation.
    ///
    /// ## Security: Domain Separation
    /// This function enforces the same prefix-based domain separation as ProofV0:
    /// - Leaf nodes are hashed as: SHA256(0x00 || borsh_serialized_leaf_data)
    /// - Internal nodes are hashed as: SHA256(0x01 || sorted_children...)
    /// - Child hashes are sorted lexicographically before concatenation.
    ///
    /// ## 256-ary Tree Structure
    /// Unlike binary trees where each level has exactly one sibling hash, 256-ary trees
    /// can have up to 255 sibling hashes per level. Each level in the proof contains
    /// all sibling hashes needed to reconstruct the parent node at that level.
    ///
    /// ## Verification Algorithm
    /// 1. Start with the leaf hash
    /// 2. For each level in the proof:
    ///    - Combine the current hash with all sibling hashes at this level
    ///    - Sort all hashes lexicographically
    ///    - Hash them together with the internal node prefix (0x01)
    ///    - Result becomes the computed hash for the next level
    /// 3. Final computed hash should equal the provided root
    pub fn verify(&self, root: &[u8; 32], leaf: &ClaimLeaf) -> bool {
        let mut computed_hash = leaf.to_hash();

        // Process each level of the proof from bottom to top
        for level_siblings in self.0.iter() {
            // Collect all hashes at this level: current computed hash + all siblings
            let mut level_hashes = Vec::with_capacity(level_siblings.len() + 1);
            level_hashes.push(computed_hash);
            level_hashes.extend_from_slice(level_siblings);

            // Sort all hashes lexicographically for deterministic ordering
            level_hashes.sort();

            // Hash the sorted children to get the parent node hash
            let mut hasher = SolanaHasher::default();
            hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]); // Internal node prefix - provides domain separation from leaf nodes (0x00)

            for child_hash in level_hashes {
                hasher.hash(&child_hash);
            }

            computed_hash = hasher.result().to_bytes();
        }

        computed_hash == *root
    }
}

// Implement From traits for easy conversion from raw data
impl From<Vec<[u8; 32]>> for ClaimProofV0 {
    fn from(proof: Vec<[u8; 32]>) -> Self {
        Self::new(proof)
    }
}

impl From<Vec<Vec<[u8; 32]>>> for ClaimProofV1 {
    fn from(proof: Vec<Vec<[u8; 32]>>) -> Self {
        Self::new(proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_leaf() -> ClaimLeaf {
        ClaimLeaf {
            campaign: Pubkey::new_unique(),
            claimant: Pubkey::new_unique(),
            vault_index: 0,
            entitlements: 100,
        }
    }

    #[test]
    fn test_proof_types_basic_functionality() {
        // Test ProofV0 (binary tree proof)
        let binary_proof_data = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let proof_v0 = ClaimProofV0::new(binary_proof_data.clone());

        assert_eq!(proof_v0.len(), 3);
        assert_eq!(proof_v0.as_slice(), &binary_proof_data);
        assert!(!proof_v0.is_empty());

        let recovered_data = proof_v0.clone().into_inner();
        assert_eq!(recovered_data, binary_proof_data);

        // Test ProofV1 (256-ary tree proof)
        let wide_proof_data = vec![
            vec![[4u8; 32], [5u8; 32]],            // Level 0: 2 siblings
            vec![[6u8; 32]],                       // Level 1: 1 sibling
            vec![[7u8; 32], [8u8; 32], [9u8; 32]], // Level 2: 3 siblings
        ];
        let proof_v1 = ClaimProofV1::new(wide_proof_data.clone());

        assert_eq!(proof_v1.len(), 3); // 3 levels
        assert_eq!(proof_v1.total_hashes(), 6); // 2 + 1 + 3 = 6 total hashes
        assert_eq!(proof_v1.max_level_width(), 3); // Level 2 has 3 siblings
        assert!(!proof_v1.is_empty());

        let recovered_wide_data = proof_v1.clone().into_inner();
        assert_eq!(recovered_wide_data, wide_proof_data);

        // Test From trait conversions
        let proof_v0_from: ClaimProofV0 = binary_proof_data.into();
        assert_eq!(proof_v0_from.len(), 3);

        let proof_v1_from: ClaimProofV1 = wide_proof_data.into();
        assert_eq!(proof_v1_from.len(), 3);
    }

    #[test]
    fn test_proof_serialization() {
        // Test that proof types can be serialized/deserialized with Borsh
        let binary_proof = ClaimProofV0::new(vec![[1u8; 32], [2u8; 32]]);
        let serialized = binary_proof.try_to_vec().unwrap();
        let deserialized: ClaimProofV0 = ClaimProofV0::try_from_slice(&serialized).unwrap();
        assert_eq!(binary_proof, deserialized);

        let wide_proof = ClaimProofV1::new(vec![vec![[3u8; 32]], vec![[4u8; 32], [5u8; 32]]]);
        let serialized = wide_proof.try_to_vec().unwrap();
        let deserialized: ClaimProofV1 = ClaimProofV1::try_from_slice(&serialized).unwrap();
        assert_eq!(wide_proof, deserialized);
    }

    /// Example showing how proof types prevent accidental misuse.
    /// This demonstrates the benefit of type safety - you can't accidentally
    /// pass a V0 proof to a function expecting V1 proofs.
    fn example_verify_binary_proof(
        proof: &ClaimProofV0,
        leaf: &ClaimLeaf,
        root: &[u8; 32],
    ) -> bool {
        proof.verify(root, leaf)
    }

    fn example_verify_wide_proof(proof: &ClaimProofV1, leaf: &ClaimLeaf, root: &[u8; 32]) -> bool {
        proof.verify(root, leaf)
    }

    #[test]
    fn test_proof_type_safety_example() {
        let leaf = create_test_leaf();
        let root = [42u8; 32];

        let binary_proof = ClaimProofV0::new(vec![[1u8; 32]]);
        let wide_proof = ClaimProofV1::new(vec![vec![[2u8; 32]]]);

        // This works - correct proof type for each function
        let _result1 = example_verify_binary_proof(&binary_proof, &leaf, &root);
        let _result2 = example_verify_wide_proof(&wide_proof, &leaf, &root);

        // These would cause compile errors (uncomment to see):
        // example_verify_binary_proof(&wide_proof, &leaf, &root);  // ❌ Type error!
        // example_verify_wide_proof(&binary_proof, &leaf, &root);  // ❌ Type error!

        // This demonstrates the value of the type system - it prevents
        // accidentally mixing proof formats at compile time.
    }

    #[test]
    fn test_proof_v1_verification_simple() {
        // Test basic 256-ary tree verification with a manually constructed example
        let leaf = create_test_leaf();
        let leaf_hash = leaf.to_hash();

        // Create a simple 1-level proof (leaf + siblings -> root)
        let sibling_1 = [1u8; 32];
        let sibling_2 = [2u8; 32];
        let sibling_3 = [3u8; 32];

        // Manually compute what the root should be
        let mut all_hashes = vec![leaf_hash, sibling_1, sibling_2, sibling_3];
        all_hashes.sort(); // Must sort like the verification algorithm

        let mut root_hasher = SolanaHasher::default();
        root_hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]); // Internal node prefix
        for hash in all_hashes {
            root_hasher.hash(&hash);
        }
        let expected_root = root_hasher.result().to_bytes();

        // Create proof with these siblings
        let proof_v1 = ClaimProofV1::new(vec![
            vec![sibling_1, sibling_2, sibling_3], // Level 0: 3 siblings
        ]);

        // Verification should succeed
        assert!(
            proof_v1.verify(&expected_root, &leaf),
            "ProofV1 verification should succeed"
        );

        // Verification should fail with wrong root
        let wrong_root = [99u8; 32];
        assert!(
            !proof_v1.verify(&wrong_root, &leaf),
            "ProofV1 verification should fail with wrong root"
        );
    }

    #[test]
    fn test_proof_v1_verification_multi_level() {
        // Test multi-level 256-ary tree verification
        let leaf = create_test_leaf();
        let leaf_hash = leaf.to_hash();

        // Level 0: leaf + 2 siblings -> level_1_hash
        let level_0_sibling_1 = [10u8; 32];
        let level_0_sibling_2 = [11u8; 32];

        let mut level_0_hashes = vec![leaf_hash, level_0_sibling_1, level_0_sibling_2];
        level_0_hashes.sort();

        let mut level_1_hasher = SolanaHasher::default();
        level_1_hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]);
        for hash in level_0_hashes {
            level_1_hasher.hash(&hash);
        }
        let level_1_hash = level_1_hasher.result().to_bytes();

        // Level 1: level_1_hash + 1 sibling -> root
        let level_1_sibling = [20u8; 32];

        let mut level_1_all_hashes = vec![level_1_hash, level_1_sibling];
        level_1_all_hashes.sort();

        let mut root_hasher = SolanaHasher::default();
        root_hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]);
        for hash in level_1_all_hashes {
            root_hasher.hash(&hash);
        }
        let expected_root = root_hasher.result().to_bytes();

        // Create 2-level proof
        let proof_v1 = ClaimProofV1::new(vec![
            vec![level_0_sibling_1, level_0_sibling_2], // Level 0: 2 siblings
            vec![level_1_sibling],                      // Level 1: 1 sibling
        ]);

        // Verification should succeed
        assert!(
            proof_v1.verify(&expected_root, &leaf),
            "Multi-level ProofV1 verification should succeed"
        );
    }

    #[test]
    fn test_proof_v1_empty_levels() {
        // Test that verification works with empty levels (no siblings at some levels)
        let leaf = create_test_leaf();
        let leaf_hash = leaf.to_hash();

        // Level 0: just the leaf (no siblings) -> level_1_hash
        let level_0_hashes = vec![leaf_hash];
        let mut level_1_hasher = SolanaHasher::default();
        level_1_hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]);
        for hash in level_0_hashes {
            level_1_hasher.hash(&hash);
        }
        let level_1_hash = level_1_hasher.result().to_bytes();

        // Level 1: level_1_hash + 1 sibling -> root
        let level_1_sibling = [25u8; 32];
        let mut level_1_all_hashes = vec![level_1_hash, level_1_sibling];
        level_1_all_hashes.sort();

        let mut root_hasher = SolanaHasher::default();
        root_hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]);
        for hash in level_1_all_hashes {
            root_hasher.hash(&hash);
        }
        let expected_root = root_hasher.result().to_bytes();

        // Create proof with empty first level
        let proof_v1 = ClaimProofV1::new(vec![
            vec![],                // Level 0: no siblings (empty)
            vec![level_1_sibling], // Level 1: 1 sibling
        ]);

        // Verification should succeed
        assert!(
            proof_v1.verify(&expected_root, &leaf),
            "ProofV1 verification should work with empty levels"
        );
    }

    #[test]
    fn test_proof_v1_hash_ordering_deterministic() {
        // Test that hash ordering is deterministic regardless of sibling order in proof
        let leaf = create_test_leaf();
        let leaf_hash = leaf.to_hash();

        let sibling_a = [50u8; 32];
        let sibling_b = [51u8; 32];
        let sibling_c = [52u8; 32];

        // Manually compute expected root (same regardless of input order)
        let mut sorted_hashes = vec![leaf_hash, sibling_a, sibling_b, sibling_c];
        sorted_hashes.sort();

        let mut root_hasher = SolanaHasher::default();
        root_hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]);
        for hash in sorted_hashes {
            root_hasher.hash(&hash);
        }
        let expected_root = root_hasher.result().to_bytes();

        // Test different orderings of siblings in proof
        let proof_order_1 = ClaimProofV1::new(vec![vec![sibling_a, sibling_b, sibling_c]]);
        let proof_order_2 = ClaimProofV1::new(vec![vec![sibling_c, sibling_a, sibling_b]]);
        let proof_order_3 = ClaimProofV1::new(vec![vec![sibling_b, sibling_c, sibling_a]]);

        // All should verify successfully (ordering shouldn't matter)
        assert!(
            proof_order_1.verify(&expected_root, &leaf),
            "ProofV1 should work regardless of sibling order 1"
        );
        assert!(
            proof_order_2.verify(&expected_root, &leaf),
            "ProofV1 should work regardless of sibling order 2"
        );
        assert!(
            proof_order_3.verify(&expected_root, &leaf),
            "ProofV1 should work regardless of sibling order 3"
        );
    }

    #[test]
    fn test_proof_v1_edge_cases() {
        let leaf = create_test_leaf();

        // Test empty proof (leaf is root)
        let empty_proof = ClaimProofV1::new(vec![]);
        let leaf_hash = leaf.to_hash();
        assert!(
            empty_proof.verify(&leaf_hash, &leaf),
            "Empty proof should verify when leaf hash equals root"
        );

        // Test single empty level
        let single_empty_level = ClaimProofV1::new(vec![vec![]]);
        let mut hasher = SolanaHasher::default();
        hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]);
        hasher.hash(&leaf_hash);
        let single_level_root = hasher.result().to_bytes();
        assert!(
            single_empty_level.verify(&single_level_root, &leaf),
            "Single empty level should work"
        );
    }
}

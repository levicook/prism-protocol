use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::Hasher as SolanaHasher;
use std::collections::HashMap;

use crate::{claim_tree_constants, consistent_hash_vault_assignment, ClaimLeaf, ClaimProofV1};

/// Creates a clean 256-ary merkle tree for claim leaves.
///
/// This implementation builds a proper 256-ary tree structure and generates
/// proofs compatible with ClaimProofV1::verify on-chain verification.
///
/// ## Arguments
///
/// * `claimant_entitlements` - List of (claimant pubkey, entitlements) pairs
/// * `vault_count` - Number of vaults for consistent assignment
///
/// ## Returns
///
/// A ClaimTreeV1 object with optimized 256-ary tree structure
pub fn create_claim_tree_v1(
    campaign: Pubkey,
    claimant_entitlements: &[(Pubkey, u64)],
    vault_count: u8,
) -> Result<ClaimTreeV1> {
    require!(!claimant_entitlements.is_empty(), ErrorCode::EmptyTree);
    require!(vault_count > 0, ErrorCode::InvalidInput);

    let claim_leaves: Vec<ClaimLeaf> = claimant_entitlements
        .iter()
        .map(|(claimant, entitlements)| {
            let vault_index = consistent_hash_vault_assignment(claimant, vault_count);
            ClaimLeaf {
                campaign,
                claimant: *claimant,
                vault_index: vault_index as u8,
                entitlements: *entitlements,
            }
        })
        .collect();

    ClaimTreeV1::from_leaves(claim_leaves)
}

/// A clean 256-ary merkle tree implementation for claim leaves.
///
/// This tree uses a width of 256 children per internal node, which provides
/// optimal balance between proof size and tree depth for on-chain verification.
///
/// ## Key Properties
///
/// - **Width**: 256 children per internal node
/// - **Hash function**: SHA256 with domain separation (0x01 prefix for internal nodes)
/// - **Ordering**: Lexicographic ordering of child hashes for deterministic results
/// - **Proof format**: Compatible with ClaimProofV1::verify
///
/// ## Security
///
/// - Uses domain separation to prevent hash collision attacks
/// - Deterministic hash ordering prevents proof malleability
/// - Compatible with existing on-chain verification logic
///
/// ## Performance
///
/// - No unnecessary dependencies (verkle trees removed)
/// - Efficient proof generation and verification
/// - Minimal memory footprint
pub struct ClaimTreeV1 {
    /// The root hash of the 256-ary tree
    root_hash: [u8; 32],
    /// Mapping from claimant pubkey to leaf index for fast lookups
    pub claimant_leaf_index: HashMap<Pubkey, usize>,
    /// Original claim leaves for verification
    pub leaves: Vec<ClaimLeaf>,
    /// Leaf hashes in tree order
    leaf_hashes: Vec<[u8; 32]>,
}

impl ClaimTreeV1 {
    /// Creates a new ClaimTreeV1 from claim leaves.
    ///
    /// Builds a complete 256-ary tree structure and computes the root hash.
    pub fn from_leaves(claim_leaves: Vec<ClaimLeaf>) -> Result<Self> {
        require!(!claim_leaves.is_empty(), ErrorCode::EmptyTree);

        // Check for duplicate claimants
        let mut seen_claimants = std::collections::HashSet::new();
        for leaf in &claim_leaves {
            require!(
                seen_claimants.insert(leaf.claimant),
                ErrorCode::DuplicateClaimant
            );
        }

        // Build leaf hashes
        let leaf_hashes: Vec<[u8; 32]> = claim_leaves.iter().map(|leaf| leaf.to_hash()).collect();

        // Compute root hash using clean 256-ary tree algorithm
        let root_hash = if leaf_hashes.len() == 1 {
            // Single leaf - root is the leaf hash
            leaf_hashes[0]
        } else {
            Self::compute_tree_root(&leaf_hashes)?
        };

        // Build claimant mapping for fast lookups
        let claimant_to_index: HashMap<Pubkey, usize> = claim_leaves
            .iter()
            .enumerate()
            .map(|(index, leaf)| (leaf.claimant, index))
            .collect();

        Ok(Self {
            root_hash,
            claimant_leaf_index: claimant_to_index,
            leaves: claim_leaves,
            leaf_hashes,
        })
    }

    /// Compute the root hash of a 256-ary tree.
    ///
    /// This function recursively builds the tree by grouping leaves into chunks
    /// of up to 256, hashing each chunk to create the next level, and continuing
    /// until a single root hash remains.
    fn compute_tree_root(leaf_hashes: &[[u8; 32]]) -> Result<[u8; 32]> {
        if leaf_hashes.len() == 1 {
            return Ok(leaf_hashes[0]);
        }

        if leaf_hashes.len() <= claim_tree_constants::BRANCHING_FACTOR {
            // All leaves fit in one internal node - compute the parent hash
            return Self::hash_internal_node(leaf_hashes);
        }

        // Multiple chunks needed - build next level
        let next_level: Result<Vec<[u8; 32]>> = leaf_hashes
            .chunks(claim_tree_constants::BRANCHING_FACTOR)
            .map(|chunk| Self::hash_internal_node(chunk))
            .collect();

        let next_level = next_level?;
        Self::compute_tree_root(&next_level)
    }

    /// Hash an internal node with its children.
    ///
    /// Uses SHA256 with domain separation (0x01 prefix) and lexicographic
    /// ordering of child hashes for deterministic results.
    fn hash_internal_node(child_hashes: &[[u8; 32]]) -> Result<[u8; 32]> {
        // Sort child hashes for deterministic ordering
        let mut sorted_children = child_hashes.to_vec();
        sorted_children.sort();

        // Hash with internal node domain separation
        let mut hasher = SolanaHasher::default();
        hasher.hash(&[claim_tree_constants::INTERNAL_PREFIX]); // Internal node prefix
        for child_hash in sorted_children {
            hasher.hash(&child_hash);
        }

        Ok(hasher.result().to_bytes())
    }

    /// Returns the root hash of the tree.
    pub fn root(&self) -> Option<[u8; 32]> {
        Some(self.root_hash)
    }

    /// Generate a 256-ary merkle proof for a specific claimant.
    ///
    /// Returns a vector of vectors, where each inner vector contains the sibling
    /// hashes needed at that level of the tree to reconstruct the parent.
    pub fn proof_for_claimant(&self, claimant: &Pubkey) -> Result<Vec<Vec<[u8; 32]>>> {
        let index = self
            .claimant_leaf_index
            .get(claimant)
            .ok_or(ErrorCode::ClaimantNotFound)?;

        self.generate_proof(*index)
    }

    /// Generate a proof for a leaf at the given index.
    ///
    /// This follows the same tree structure as the root computation, collecting
    /// sibling hashes at each level needed to reconstruct the path to the root.
    fn generate_proof(&self, leaf_index: usize) -> Result<Vec<Vec<[u8; 32]>>> {
        if self.leaf_hashes.len() == 1 {
            // Single leaf - empty proof
            return Ok(vec![]);
        }

        let mut proof = Vec::new();
        let mut current_hashes = self.leaf_hashes.clone();
        let mut current_index = leaf_index;

        while current_hashes.len() > 1 {
            // Find which chunk contains our current index
            let chunk_index = current_index / claim_tree_constants::BRANCHING_FACTOR;
            let position_in_chunk = current_index % claim_tree_constants::BRANCHING_FACTOR;

            // Get all chunks at this level
            let chunks: Vec<&[[u8; 32]]> = current_hashes
                .chunks(claim_tree_constants::BRANCHING_FACTOR)
                .collect();

            if chunk_index >= chunks.len() {
                return Err(error!(ErrorCode::InvalidIndex));
            }

            // Collect sibling hashes within this chunk
            let chunk = chunks[chunk_index];
            let mut siblings = Vec::new();

            for (i, &hash) in chunk.iter().enumerate() {
                if i != position_in_chunk {
                    siblings.push(hash);
                }
            }

            proof.push(siblings);

            // Build next level by hashing each chunk
            let next_level: Result<Vec<[u8; 32]>> = current_hashes
                .chunks(claim_tree_constants::BRANCHING_FACTOR)
                .map(|chunk| Self::hash_internal_node(chunk))
                .collect();

            current_hashes = next_level?;
            current_index = chunk_index;
        }

        Ok(proof)
    }

    /// Generate merkle proofs for multiple claimants.
    pub fn proofs_for_claimants(
        &self,
        claimants: &[Pubkey],
    ) -> Result<HashMap<Pubkey, Vec<Vec<[u8; 32]>>>> {
        let mut proofs = HashMap::new();
        for claimant in claimants {
            let proof = self.proof_for_claimant(claimant)?;
            proofs.insert(*claimant, proof);
        }
        Ok(proofs)
    }

    /// Get the leaf data for a specific claimant
    pub fn leaf_for_claimant(&self, claimant: &Pubkey) -> Result<&ClaimLeaf> {
        let index = self
            .claimant_leaf_index
            .get(claimant)
            .ok_or(ErrorCode::ClaimantNotFound)?;

        self.leaves
            .get(*index)
            .ok_or_else(|| error!(ErrorCode::InvalidIndex))
    }

    /// Verify a 256-ary merkle proof for a given claimant.
    ///
    /// This uses the existing ClaimProofV1::verify logic for compatibility.
    pub fn verify_proof(&self, claimant: &Pubkey, proof: &[Vec<[u8; 32]>]) -> Result<bool> {
        let leaf = self.leaf_for_claimant(claimant)?;
        let root = self.root().ok_or(ErrorCode::MissingMerkleRoot)?;

        // Use the ClaimProofV1 verification logic for compatibility
        let claim_proof = ClaimProofV1::new(proof.to_vec());
        Ok(claim_proof.verify(&root, leaf))
    }

    /// Creates a ClaimProofV1 for a specific claimant.
    pub fn create_proof(&self, claimant: &Pubkey) -> Result<ClaimProofV1> {
        let siblings = self.proof_for_claimant(claimant)?;
        Ok(ClaimProofV1::new(siblings))
    }

    /// Returns the number of leaves in the tree.
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    /// Returns the depth of the tree (number of levels from leaf to root).
    pub fn depth(&self) -> usize {
        if self.leaves.len() <= 1 {
            return 0;
        }

        // Calculate depth based on 256-ary tree structure
        let mut remaining = self.leaves.len();
        let mut depth = 0;

        while remaining > 1 {
            remaining = (remaining + claim_tree_constants::BRANCHING_FACTOR - 1)
                / claim_tree_constants::BRANCHING_FACTOR; // Ceiling division
            depth += 1;
        }

        depth
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Tree cannot be empty")]
    EmptyTree,
    #[msg("Duplicate claimant found")]
    DuplicateClaimant,
    #[msg("Claimant not found")]
    ClaimantNotFound,
    #[msg("Invalid input")]
    InvalidInput,
    #[msg("Invalid index")]
    InvalidIndex,
    #[msg("Missing merkle root")]
    MissingMerkleRoot,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_leaf(claimant_seed: u8, entitlements: u64) -> ClaimLeaf {
        let campaign = Pubkey::new_unique();
        let mut claimant_bytes = [0u8; 32];
        claimant_bytes[0] = claimant_seed;
        let claimant = Pubkey::new_from_array(claimant_bytes);
        ClaimLeaf {
            campaign,
            claimant,
            entitlements,
            vault_index: 0,
        }
    }

    fn create_test_leaf_unique(index: usize, entitlements: u64) -> ClaimLeaf {
        let campaign = Pubkey::new_unique();
        let mut claimant_bytes = [0u8; 32];
        // Use multiple bytes to avoid duplicates for large indices
        claimant_bytes[0] = (index & 0xFF) as u8;
        claimant_bytes[1] = ((index >> 8) & 0xFF) as u8;
        claimant_bytes[2] = ((index >> 16) & 0xFF) as u8;
        claimant_bytes[3] = ((index >> 24) & 0xFF) as u8;
        let claimant = Pubkey::new_from_array(claimant_bytes);
        ClaimLeaf {
            campaign,
            claimant,
            entitlements,
            vault_index: 0,
        }
    }

    #[test]
    fn test_single_leaf_tree() {
        let leaf = create_test_leaf(1, 100);
        let tree = ClaimTreeV1::from_leaves(vec![leaf.clone()]).unwrap();

        // Single leaf - root should be leaf hash
        assert_eq!(tree.root(), Some(leaf.to_hash()));
        assert_eq!(tree.leaf_count(), 1);
        assert_eq!(tree.depth(), 0);

        // Empty proof for single leaf
        let proof = tree.proof_for_claimant(&leaf.claimant).unwrap();
        assert!(proof.is_empty());

        // Verification should work
        let verification_result = tree.verify_proof(&leaf.claimant, &proof).unwrap();
        assert!(verification_result);
    }

    #[test]
    fn test_small_tree() {
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
        ];

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();

        assert_eq!(tree.leaf_count(), 3);
        assert_eq!(tree.depth(), 1); // Single level above leaves

        // Test proofs for all leaves
        for leaf in &leaves {
            let proof = tree.proof_for_claimant(&leaf.claimant).unwrap();
            assert_eq!(proof.len(), 1); // One level of proof
            assert_eq!(proof[0].len(), 2); // Two siblings

            let verification_result = tree.verify_proof(&leaf.claimant, &proof).unwrap();
            assert!(verification_result);

            // Test ClaimProofV1 compatibility
            let claim_proof = tree.create_proof(&leaf.claimant).unwrap();
            assert!(claim_proof.verify(&tree.root().unwrap(), leaf));
        }
    }

    #[test]
    fn test_large_tree() {
        // Test with 300 leaves to verify multi-level structure
        let leaves: Vec<ClaimLeaf> = (0..300)
            .map(|i| create_test_leaf_unique(i, (i + 1) as u64 * 10))
            .collect();

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();

        assert_eq!(tree.leaf_count(), 300);
        assert_eq!(tree.depth(), 2); // 300 leaves require 2 levels above leaves

        // Test first few and last few proofs
        let test_indices = [0, 1, 2, 255, 256, 299];
        for &i in &test_indices {
            let leaf = &leaves[i];
            let proof = tree.proof_for_claimant(&leaf.claimant).unwrap();

            let verification_result = tree.verify_proof(&leaf.claimant, &proof).unwrap();
            assert!(
                verification_result,
                "Proof verification failed for leaf {}",
                i
            );

            // Test ClaimProofV1 compatibility
            let claim_proof = tree.create_proof(&leaf.claimant).unwrap();
            assert!(claim_proof.verify(&tree.root().unwrap(), leaf));
        }
    }

    #[test]
    fn test_tree_deterministic() {
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
        ];

        // Build same tree multiple times
        let tree1 = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
        let tree2 = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();

        // Should have identical roots
        assert_eq!(tree1.root(), tree2.root());

        // Should have identical proofs
        for leaf in &leaves {
            let proof1 = tree1.proof_for_claimant(&leaf.claimant).unwrap();
            let proof2 = tree2.proof_for_claimant(&leaf.claimant).unwrap();
            assert_eq!(proof1, proof2);
        }
    }

    #[test]
    fn test_duplicate_claimant_error() {
        let campaign = Pubkey::new_unique();
        let claimant_bytes = [1u8; 32];
        let claimant = Pubkey::new_from_array(claimant_bytes);

        let leaves = vec![
            ClaimLeaf {
                campaign,
                claimant,
                entitlements: 100,
                vault_index: 0,
            },
            ClaimLeaf {
                campaign,
                claimant, // Duplicate!
                entitlements: 200,
                vault_index: 1,
            },
        ];

        let result = ClaimTreeV1::from_leaves(leaves);
        assert!(result.is_err());
    }

    #[test]
    fn test_nonexistent_claimant() {
        let leaves = vec![create_test_leaf(1, 100)];
        let tree = ClaimTreeV1::from_leaves(leaves).unwrap();

        let fake_claimant = Pubkey::new_unique();
        let result = tree.proof_for_claimant(&fake_claimant);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_claim_tree_v1_function() {
        let campaign = Pubkey::new_unique();
        let claimants = vec![
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        let vault_count = 3;

        let tree = create_claim_tree_v1(
            campaign,
            &claimants.iter().map(|&c| (c, 100)).collect::<Vec<_>>(),
            vault_count,
        )
        .unwrap();

        // Test basic functionality
        for claimant in &claimants {
            let proof = tree.proof_for_claimant(claimant).unwrap();
            let verification_result = tree.verify_proof(claimant, &proof).unwrap();
            assert!(verification_result);
        }

        assert_ne!(tree.root(), None);
    }

    #[test]
    fn test_batch_proofs() {
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
        ];

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
        let claimants: Vec<Pubkey> = leaves.iter().map(|l| l.claimant).collect();

        let batch_proofs = tree.proofs_for_claimants(&claimants).unwrap();

        assert_eq!(batch_proofs.len(), 3);

        // Verify all batch proofs
        for (claimant, proof) in batch_proofs {
            let verification_result = tree.verify_proof(&claimant, &proof).unwrap();
            assert!(verification_result);
        }
    }

    #[test]
    fn test_proof_format_compatibility() {
        // Test that our proofs work with the existing ClaimProofV1::verify logic
        let leaves = vec![create_test_leaf(1, 100), create_test_leaf(2, 200)];

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();

        for leaf in leaves.iter() {
            let proof_siblings = tree.proof_for_claimant(&leaf.claimant).unwrap();
            let claim_proof = ClaimProofV1::new(proof_siblings);
            let root = tree.root().unwrap();

            // This uses the exact same verification logic as on-chain
            assert!(
                claim_proof.verify(&root, leaf),
                "ClaimProofV1 verification should work"
            );
        }
    }

    // =================================================================
    // COMPREHENSIVE ClaimProofV1::verify TESTS
    // =================================================================
    // These tests thoroughly exercise ClaimProofV1::verify using real
    // roots and proofs generated by our clean ClaimTreeV1 implementation

    #[test]
    fn test_claim_proof_v1_verify_single_leaf() {
        // Test: Single leaf tree (empty proof)
        let leaf = create_test_leaf(42, 1000);
        let tree = ClaimTreeV1::from_leaves(vec![leaf.clone()]).unwrap();
        let root = tree.root().unwrap();
        let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();

        let proof = ClaimProofV1::new(proof_data);

        // Single leaf should have empty proof and root == leaf.to_hash()
        assert!(proof.is_empty(), "Single leaf proof should be empty");
        assert_eq!(
            root,
            leaf.to_hash(),
            "Single leaf root should equal leaf hash"
        );
        assert!(
            proof.verify(&root, &leaf),
            "Single leaf proof should verify"
        );
    }

    #[test]
    fn test_claim_proof_v1_verify_small_trees() {
        // Test: Various small tree sizes
        for num_leaves in 2..=10 {
            let leaves: Vec<ClaimLeaf> = (0..num_leaves)
                .map(|i| create_test_leaf_unique(i, (i + 1) as u64 * 100))
                .collect();

            let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
            let root = tree.root().unwrap();

            // Test every leaf's proof
            for leaf in &leaves {
                let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
                let proof = ClaimProofV1::new(proof_data);

                assert!(
                    proof.verify(&root, leaf),
                    "Proof should verify for tree with {} leaves",
                    num_leaves
                );
            }
        }
    }

    #[test]
    fn test_claim_proof_v1_verify_boundary_conditions() {
        // Test: Exactly 256 leaves (fits in one internal node)
        let leaves: Vec<ClaimLeaf> = (0..256)
            .map(|i| create_test_leaf_unique(i, (i + 1) as u64))
            .collect();

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
        let root = tree.root().unwrap();

        // Test first, middle, and last leaves
        let test_indices = [0, 128, 255];
        for &i in &test_indices {
            let leaf = &leaves[i];
            let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
            let proof = ClaimProofV1::new(proof_data);

            // Should have exactly 1 level with 255 siblings
            assert_eq!(proof.len(), 1, "256 leaves should have 1-level proof");
            assert_eq!(proof.as_slice()[0].len(), 255, "Should have 255 siblings");

            assert!(
                proof.verify(&root, leaf),
                "Proof should verify for leaf {} in 256-leaf tree",
                i
            );
        }

        // Test: 257 leaves (requires 2 levels)
        let leaves_257: Vec<ClaimLeaf> = (0..257)
            .map(|i| create_test_leaf_unique(i, (i + 1) as u64))
            .collect();

        let tree_257 = ClaimTreeV1::from_leaves(leaves_257.clone()).unwrap();
        let root_257 = tree_257.root().unwrap();

        // Test leaves in first chunk (0-255) and second chunk (256)
        let test_indices_257 = [0, 255, 256];
        for &i in &test_indices_257 {
            let leaf = &leaves_257[i];
            let proof_data = tree_257.proof_for_claimant(&leaf.claimant).unwrap();
            let proof = ClaimProofV1::new(proof_data);

            // Should have exactly 2 levels
            assert_eq!(proof.len(), 2, "257 leaves should have 2-level proof");

            assert!(
                proof.verify(&root_257, leaf),
                "Proof should verify for leaf {} in 257-leaf tree",
                i
            );
        }
    }

    #[test]
    fn test_claim_proof_v1_verify_large_tree() {
        // Test: Large tree with 1000 leaves (3 levels deep)
        let leaves: Vec<ClaimLeaf> = (0..1000)
            .map(|i| create_test_leaf_unique(i, (i + 1) as u64 * 10))
            .collect();

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
        let root = tree.root().unwrap();

        // Test leaves across different chunks and levels
        let test_indices = [0, 1, 255, 256, 511, 512, 767, 768, 999];
        for &i in &test_indices {
            let leaf = &leaves[i];
            let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
            let proof = ClaimProofV1::new(proof_data);

            assert!(
                proof.verify(&root, leaf),
                "Proof should verify for leaf {} in 1000-leaf tree",
                i
            );
        }

        // Verify tree structure
        assert_eq!(tree.depth(), 2, "1000 leaves should create 2-level tree");
    }

    #[test]
    fn test_claim_proof_v1_verify_wrong_root() {
        // Test: Valid proof but wrong root should fail
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
        ];

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
        let _correct_root = tree.root().unwrap();
        let wrong_root = [0xFF; 32]; // Obviously wrong root

        for leaf in &leaves {
            let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
            let proof = ClaimProofV1::new(proof_data);

            assert!(
                !proof.verify(&wrong_root, leaf),
                "Proof should fail with wrong root"
            );
        }
    }

    #[test]
    fn test_claim_proof_v1_verify_wrong_leaf() {
        // Test: Valid proof and root but wrong leaf should fail
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
        ];

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
        let root = tree.root().unwrap();

        // Create a fake leaf not in the tree
        let fake_leaf = create_test_leaf(99, 999);

        for leaf in &leaves {
            let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
            let proof = ClaimProofV1::new(proof_data);

            assert!(
                !proof.verify(&root, &fake_leaf),
                "Proof should fail with wrong leaf data"
            );
        }
    }

    #[test]
    fn test_claim_proof_v1_verify_corrupted_proof() {
        // Test: Corrupted proof data should fail verification
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
        ];

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
        let root = tree.root().unwrap();
        let leaf = &leaves[0];

        // Get valid proof and corrupt it
        let mut proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();

        // Corrupt first sibling hash in first level
        if !proof_data.is_empty() && !proof_data[0].is_empty() {
            proof_data[0][0] = [0xFF; 32]; // Corrupt the hash

            let corrupted_proof = ClaimProofV1::new(proof_data);
            assert!(
                !corrupted_proof.verify(&root, leaf),
                "Corrupted proof should fail verification"
            );
        }
    }

    #[test]
    fn test_claim_proof_v1_verify_empty_proof_wrong_context() {
        // Test: Empty proof should only work when root == leaf.to_hash()
        let leaf = create_test_leaf(1, 100);
        let empty_proof = ClaimProofV1::new(vec![]);

        // Should work when root equals leaf hash
        let correct_root = leaf.to_hash();
        assert!(
            empty_proof.verify(&correct_root, &leaf),
            "Empty proof should verify when root == leaf.to_hash()"
        );

        // Should fail with any other root
        let wrong_root = [0xFF; 32];
        assert!(
            !empty_proof.verify(&wrong_root, &leaf),
            "Empty proof should fail when root != leaf.to_hash()"
        );
    }

    #[test]
    fn test_claim_proof_v1_verify_proof_malleability_resistance() {
        // Test: Different orderings of siblings should all work (deterministic sorting)
        let leaves = vec![
            create_test_leaf(1, 100),
            create_test_leaf(2, 200),
            create_test_leaf(3, 300),
        ];

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
        let root = tree.root().unwrap();
        let leaf = &leaves[0];

        let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();

        if !proof_data.is_empty() && proof_data[0].len() >= 2 {
            // Create proof with siblings in different order
            let mut reordered_proof_data = proof_data.clone();
            reordered_proof_data[0].reverse(); // Reverse sibling order

            let reordered_proof = ClaimProofV1::new(reordered_proof_data);

            // Should still verify because ClaimProofV1::verify sorts siblings
            assert!(
                reordered_proof.verify(&root, leaf),
                "Proof should verify regardless of sibling ordering"
            );
        }
    }

    #[test]
    fn test_claim_proof_v1_verify_domain_separation() {
        // Test: Ensure domain separation is working (internal nodes vs leaves)
        let leaves = vec![create_test_leaf(1, 100), create_test_leaf(2, 200)];

        let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
        let root = tree.root().unwrap();

        // Verify that root is NOT equal to any leaf hash (domain separation working)
        for leaf in &leaves {
            let leaf_hash = leaf.to_hash();
            assert_ne!(
                root, leaf_hash,
                "Root should never equal leaf hash due to domain separation"
            );
        }

        // Verify proofs work correctly
        for leaf in &leaves {
            let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
            let proof = ClaimProofV1::new(proof_data);

            assert!(
                proof.verify(&root, leaf),
                "Proof should verify with proper domain separation"
            );
        }
    }

    #[test]
    fn test_claim_proof_v1_verify_proof_size_analysis() {
        // Test: Analyze proof sizes for different tree sizes
        let test_sizes = [1, 2, 10, 100, 256, 257, 500, 1000];

        for &size in &test_sizes {
            let leaves: Vec<ClaimLeaf> = (0..size)
                .map(|i| create_test_leaf_unique(i, (i + 1) as u64))
                .collect();

            let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
            let root = tree.root().unwrap();

            // Test first leaf's proof
            if !leaves.is_empty() {
                let leaf = &leaves[0];
                let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
                let proof = ClaimProofV1::new(proof_data);

                // Verify the proof works
                assert!(
                    proof.verify(&root, leaf),
                    "Proof should verify for tree size {}",
                    size
                );

                // Log proof characteristics for analysis
                println!(
                    "Tree size: {}, Depth: {}, Proof levels: {}, Total hashes: {}, Max level width: {}",
                    size,
                    tree.depth(),
                    proof.len(),
                    proof.total_hashes(),
                    proof.max_level_width()
                );

                // Sanity checks on proof structure
                if size == 1 {
                    assert_eq!(proof.len(), 0, "Single leaf should have empty proof");
                } else {
                    assert!(
                        proof.len() > 0,
                        "Multi-leaf tree should have non-empty proof"
                    );
                    assert!(
                        proof.total_hashes() > 0,
                        "Multi-leaf proof should have hashes"
                    );
                }
            }
        }
    }

    #[test]
    fn test_claim_proof_v1_verify_stress_test() {
        // Test: Stress test with various tree configurations
        let configurations = [
            (50, "Small tree"),
            (255, "Just under 256"),
            (256, "Exactly 256"),
            (257, "Just over 256"),
            (512, "Two full chunks"),
            (513, "Two chunks + 1"),
            (1000, "Large tree"),
        ];

        for (size, description) in configurations {
            let leaves: Vec<ClaimLeaf> = (0..size)
                .map(|i| create_test_leaf_unique(i, (i + 1) as u64 * 123)) // Different entitlements
                .collect();

            let tree = ClaimTreeV1::from_leaves(leaves.clone()).unwrap();
            let root = tree.root().unwrap();

            // Test multiple random leaves for each configuration
            let test_indices: Vec<usize> = if size <= 10 {
                (0..size).collect()
            } else {
                vec![0, 1, size / 4, size / 2, 3 * size / 4, size - 2, size - 1]
            };

            for &i in &test_indices {
                let leaf = &leaves[i];
                let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
                let proof = ClaimProofV1::new(proof_data);

                assert!(
                    proof.verify(&root, leaf),
                    "Proof should verify for {} (leaf {} of {})",
                    description,
                    i,
                    size
                );
            }
        }
    }

    #[test]
    fn test_claim_proof_v1_verify_cross_tree_isolation() {
        // Test: Proofs from one tree should not verify against another tree's root
        let leaves_tree_a = vec![create_test_leaf(1, 100), create_test_leaf(2, 200)];

        let leaves_tree_b = vec![create_test_leaf(3, 300), create_test_leaf(4, 400)];

        let tree_a = ClaimTreeV1::from_leaves(leaves_tree_a.clone()).unwrap();
        let tree_b = ClaimTreeV1::from_leaves(leaves_tree_b.clone()).unwrap();

        let root_a = tree_a.root().unwrap();
        let root_b = tree_b.root().unwrap();

        // Ensure roots are different
        assert_ne!(
            root_a, root_b,
            "Different trees should have different roots"
        );

        // Test that tree A's proof doesn't verify against tree B's root
        for leaf in &leaves_tree_a {
            let proof_data_a = tree_a.proof_for_claimant(&leaf.claimant).unwrap();
            let proof_a = ClaimProofV1::new(proof_data_a);

            // Should verify against correct root
            assert!(
                proof_a.verify(&root_a, leaf),
                "Proof should verify against correct root"
            );

            // Should NOT verify against wrong root
            assert!(
                !proof_a.verify(&root_b, leaf),
                "Proof should NOT verify against different tree's root"
            );
        }
    }
}

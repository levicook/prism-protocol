use anchor_lang::prelude::*;
use rs_merkle::MerkleTree;
use std::collections::HashMap;

use crate::{consistent_hash_vault_assignment, ClaimHasherV0, ClaimLeaf};

/// Creates a merkle tree using consistent hashing to assign claimants to vaults.
///
/// This is the production function that implements the design goal of deterministic
/// vault assignment via consistent hashing. Each claimant is mapped to a vault index
/// (0, 1, 2, ..., vaults.len()-1) based on a hash of their pubkey.
///
/// ## Consistent Hashing Algorithm
///
/// For each claimant:
/// 1. Hash the claimant's pubkey: `SHA256(claimant_pubkey.as_ref())`
/// 2. Convert the first 8 bytes of the hash to u64 (little-endian)
/// 3. Modulo by the number of vaults to get the vault index
/// 4. Assign the claimant to `vaults[vault_index]`
///
/// This ensures:
/// - **Deterministic assignment**: Same claimant always maps to same vault
/// - **Even distribution**: Claimants are distributed roughly evenly across vaults
/// - **Immutable mapping**: Vault assignment doesn't change if vault list order is preserved
///
/// ## Parameters
/// - `claimant_entitlements`: List of (claimant_pubkey, entitlements) pairs
/// - `vault_count`: Number of vaults
///
/// ## Returns
/// A `ClaimMerkleTree` with leaves containing the assigned vault index for each claimant.
pub fn create_claim_tree_v0(
    campaign: Pubkey,
    claimant_entitlements: &[(Pubkey, u64)],
    vault_count: u8,
) -> Result<ClaimTreeV0> {
    require!(!claimant_entitlements.is_empty(), ErrorCode::InvalidInput);
    require!(vault_count > 0, ErrorCode::InvalidInput);

    let leaves: Vec<ClaimLeaf> = claimant_entitlements
        .iter()
        .map(|(claimant, entitlements)| {
            // Consistent hashing: hash the claimant pubkey to determine vault assignment
            let vault_index = consistent_hash_vault_assignment(claimant, vault_count);

            ClaimLeaf {
                campaign,
                claimant: *claimant,
                vault_index,
                entitlements: *entitlements,
            }
        })
        .collect();

    ClaimTreeV0::from_leaves(leaves)
}

/// Result of building a merkle tree from claim leaves
#[derive(Clone)]
pub struct ClaimTreeV0 {
    /// The underlying merkle tree
    pub tree: MerkleTree<ClaimHasherV0>,
    /// Mapping from claimant pubkey to their leaf index in the tree
    pub claimant_leaf_index: HashMap<Pubkey, usize>,
    /// The original leaves used to build the tree
    pub leaves: Vec<ClaimLeaf>,
}

impl ClaimTreeV0 {
    /// Build a merkle tree from a list of claim leaves
    pub fn from_leaves(leaves: Vec<ClaimLeaf>) -> Result<Self> {
        require!(!leaves.is_empty(), ErrorCode::InvalidInput);

        // Create mapping from claimant to index
        let mut claimant_to_index = HashMap::new();
        for (index, leaf) in leaves.iter().enumerate() {
            if claimant_to_index.insert(leaf.claimant, index).is_some() {
                return err!(ErrorCode::DuplicateClaimant);
            }
        }

        // Hash all leaves
        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|leaf| leaf.to_hash()).collect();

        // Build the merkle tree
        let tree = MerkleTree::<ClaimHasherV0>::from_leaves(&leaf_hashes);

        Ok(ClaimTreeV0 {
            tree,
            claimant_leaf_index: claimant_to_index,
            leaves,
        })
    }

    /// Get the merkle root
    pub fn root(&self) -> Option<[u8; 32]> {
        self.tree.root()
    }

    /// Generate a merkle proof for a specific claimant
    pub fn proof_for_claimant(&self, claimant: &Pubkey) -> Result<Vec<[u8; 32]>> {
        let index = self
            .claimant_leaf_index
            .get(claimant)
            .ok_or(ErrorCode::ClaimantNotFound)?;

        let proof = self.tree.proof(&[*index]);
        let proof_hashes = proof.proof_hashes();

        Ok(proof_hashes.to_vec())
    }

    /// Generate merkle proofs for multiple claimants
    pub fn proofs_for_claimants(
        &self,
        claimants: &[Pubkey],
    ) -> Result<HashMap<Pubkey, Vec<[u8; 32]>>> {
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

    /// Verify a proof for a given claimant
    pub fn verify_proof(&self, claimant: &Pubkey, proof: &[[u8; 32]]) -> Result<bool> {
        let root = self.root().ok_or(ErrorCode::MissingMerkleRoot)?;
        let index = self
            .claimant_leaf_index
            .get(claimant)
            .ok_or(ErrorCode::ClaimantNotFound)?;
        let leaf = self.leaf_for_claimant(claimant)?;
        let leaf_hash = leaf.to_hash();

        let merkle_proof = rs_merkle::MerkleProof::<ClaimHasherV0>::new(proof.to_vec());

        Ok(merkle_proof.verify(root, &[*index], &[leaf_hash], self.leaves.len()))
    }
}

/// Custom error codes for merkle tree operations
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid input provided")]
    InvalidInput,
    #[msg("Duplicate claimant found")]
    DuplicateClaimant,
    #[msg("Claimant not found in tree")]
    ClaimantNotFound,
    #[msg("Invalid index")]
    InvalidIndex,
    #[msg("Missing merkle root")]
    MissingMerkleRoot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_merkle_tree_from_leaves() {
        let campaign = Pubkey::new_unique();

        let claimants = [
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];

        let leaves = vec![
            ClaimLeaf {
                campaign,
                claimant: claimants[0],
                entitlements: 100,
                vault_index: 0,
            },
            ClaimLeaf {
                campaign,
                claimant: claimants[1],
                entitlements: 200,
                vault_index: 0,
            },
            ClaimLeaf {
                campaign,
                claimant: claimants[2],
                entitlements: 300,
                vault_index: 1,
            },
        ];

        let merkle_tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();

        // Verify root exists
        assert!(merkle_tree.root().is_some());

        // Verify all claimants are mapped
        for (i, claimant) in claimants.iter().enumerate() {
            assert_eq!(merkle_tree.claimant_leaf_index[claimant], i);
        }

        // Verify leaves are stored correctly
        assert_eq!(merkle_tree.leaves, leaves);
    }

    #[test]
    fn test_generate_and_verify_proof() {
        let campaign = Pubkey::new_unique();
        let claimants = [Pubkey::new_unique(), Pubkey::new_unique()];

        let leaves = vec![
            ClaimLeaf {
                campaign,
                claimant: claimants[0],
                entitlements: 100,
                vault_index: 0,
            },
            ClaimLeaf {
                campaign,
                claimant: claimants[1],
                entitlements: 200,
                vault_index: 1,
            },
        ];

        let merkle_tree = ClaimTreeV0::from_leaves(leaves).unwrap();

        // Generate proof for first claimant
        let proof = merkle_tree.proof_for_claimant(&claimants[0]).unwrap();

        // Verify the proof
        let is_valid = merkle_tree.verify_proof(&claimants[0], &proof).unwrap();
        assert!(is_valid, "Proof should be valid");

        // Verify proof fails for wrong claimant
        let is_valid = merkle_tree.verify_proof(&claimants[1], &proof).unwrap();
        assert!(!is_valid, "Proof should be invalid for wrong claimant");
    }

    #[test]
    fn test_duplicate_claimant_error() {
        let campaign = Pubkey::new_unique();
        let claimant = Pubkey::new_unique();

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

        let result = ClaimTreeV0::from_leaves(leaves);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_merkle_tree_consistent_hashing() {
        let campaign = Pubkey::new_unique();
        let claimants = [
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        let vault_count = 3;

        // Create claimant entitlements with different amounts
        let claimant_entitlements: Vec<(Pubkey, u64)> = claimants
            .iter()
            .enumerate()
            .map(|(i, &claimant)| (claimant, (i + 1) as u64 * 100))
            .collect();

        let merkle_tree =
            create_claim_tree_v0(campaign, &claimant_entitlements, vault_count).unwrap();

        // Verify tree was created successfully
        assert!(merkle_tree.root().is_some());
        assert_eq!(merkle_tree.leaves.len(), claimants.len());

        // Verify each claimant has correct entitlements and deterministic vault assignment
        for (claimant, expected_entitlements) in claimant_entitlements.iter() {
            let leaf = merkle_tree.leaf_for_claimant(claimant).unwrap();
            assert_eq!(leaf.claimant, *claimant);
            assert_eq!(leaf.entitlements, *expected_entitlements);

            // Verify consistent hashing: same claimant should always get same vault
            let vault_index = crate::consistent_hash_vault_assignment(claimant, vault_count);
            assert_eq!(leaf.vault_index, vault_index);
        }

        // Test determinism: creating the tree again should produce identical assignments
        let merkle_tree2 =
            create_claim_tree_v0(campaign, &claimant_entitlements, vault_count).unwrap();
        for (leaf1, leaf2) in merkle_tree.leaves.iter().zip(merkle_tree2.leaves.iter()) {
            assert_eq!(leaf1.claimant, leaf2.claimant);
            assert_eq!(leaf1.vault_index as usize, leaf2.vault_index as usize);
            assert_eq!(leaf1.entitlements, leaf2.entitlements);
        }
    }

    #[test]
    fn test_consistent_hash_vault_assignment_deterministic() {
        let claimant = Pubkey::new_unique();
        let vault_count = 3;

        // Same claimant should always get same vault index
        let index1 = crate::consistent_hash_vault_assignment(&claimant, vault_count);
        let index2 = crate::consistent_hash_vault_assignment(&claimant, vault_count);
        assert_eq!(index1, index2);

        // Index should be within bounds
        assert!(index1 < vault_count);
    }

    #[test]
    fn test_consistent_hash_distribution() {
        // Test that the hash function distributes claimants roughly evenly
        let vault_count = 5;
        let num_claimants = 1000;

        let mut vault_counts = vec![0; vault_count];

        for i in 0..num_claimants {
            let claimant = Pubkey::new_from_array([i as u8; 32]);
            let vault_index = crate::consistent_hash_vault_assignment(&claimant, vault_count as u8);
            vault_counts[vault_index as usize] += 1;
        }

        // Each vault should get roughly 200 claimants (1000/5)
        // Allow some variance - each should be between 150-250
        for count in vault_counts {
            assert!(count >= 150 && count <= 250, "Vault count: {}", count);
        }
    }

    // =================================================================
    // COMPREHENSIVE ClaimProofV0::verify TESTS
    // =================================================================
    // These tests thoroughly exercise ClaimProofV0::verify using real
    // roots and proofs generated by our ClaimTreeV0 implementation

    #[test]
    fn test_claim_proof_v0_verify_single_leaf() {
        // Test: Single leaf tree (empty proof)
        let campaign = Pubkey::new_unique();
        let claimant = Pubkey::new_unique();
        let leaf = ClaimLeaf {
            campaign,
            claimant,
            entitlements: 1000,
            vault_index: 0,
        };

        let tree = ClaimTreeV0::from_leaves(vec![leaf.clone()]).unwrap();
        let root = tree.root().unwrap();
        let proof_data = tree.proof_for_claimant(&claimant).unwrap();

        let proof = crate::ClaimProofV0::new(proof_data);

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
    fn test_claim_proof_v0_verify_small_trees() {
        // Test: Various small tree sizes (binary trees)
        let campaign = Pubkey::new_unique();
        for num_leaves in 2..=16 {
            let claimants: Vec<Pubkey> = (0..num_leaves).map(|_| Pubkey::new_unique()).collect();
            let leaves: Vec<ClaimLeaf> = claimants
                .iter()
                .enumerate()
                .map(|(i, &claimant)| ClaimLeaf {
                    campaign,
                    claimant,
                    entitlements: (i + 1) as u64 * 100,
                    vault_index: (i % 3) as u8,
                })
                .collect();

            let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
            let root = tree.root().unwrap();

            // Test every leaf's proof
            for leaf in &leaves {
                let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
                let proof = crate::ClaimProofV0::new(proof_data);

                assert!(
                    proof.verify(&root, leaf),
                    "Proof should verify for tree with {} leaves",
                    num_leaves
                );
            }
        }
    }

    #[test]
    fn test_claim_proof_v0_verify_boundary_conditions() {
        // Test: Power of 2 sizes (perfect binary trees)
        let campaign = Pubkey::new_unique();
        let power_of_2_sizes = [2, 4, 8, 16, 32, 64, 128];

        for &size in &power_of_2_sizes {
            let claimants: Vec<Pubkey> = (0..size).map(|_| Pubkey::new_unique()).collect();
            let leaves: Vec<ClaimLeaf> = claimants
                .iter()
                .enumerate()
                .map(|(i, &claimant)| ClaimLeaf {
                    campaign,
                    claimant,
                    vault_index: (i % 5) as u8,
                    entitlements: (i + 1) as u64 * 50,
                })
                .collect();

            let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
            let root = tree.root().unwrap();

            // Test first, middle, and last leaves
            let test_indices = if size <= 4 {
                (0..size).collect::<Vec<_>>()
            } else {
                vec![0, size / 2, size - 1]
            };

            for &i in &test_indices {
                let leaf = &leaves[i];
                let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
                let proof = crate::ClaimProofV0::new(proof_data);

                assert!(
                    proof.verify(&root, leaf),
                    "Proof should verify for leaf {} in {}-leaf perfect binary tree",
                    i,
                    size
                );
            }
        }

        // Test: Non-power of 2 sizes (imperfect binary trees)
        let campaign = Pubkey::new_unique();
        let irregular_sizes = [3, 5, 7, 15, 31, 63, 100];

        for &size in &irregular_sizes {
            let claimants: Vec<Pubkey> = (0..size).map(|_| Pubkey::new_unique()).collect();
            let leaves: Vec<ClaimLeaf> = claimants
                .iter()
                .enumerate()
                .map(|(i, &claimant)| ClaimLeaf {
                    campaign,
                    claimant,
                    entitlements: (i + 1) as u64 * 75,
                    vault_index: (i % 7) as u8,
                })
                .collect();

            let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
            let root = tree.root().unwrap();

            // Test edge cases in irregular trees
            let test_indices = vec![0, size - 1];
            for &i in &test_indices {
                let leaf = &leaves[i];
                let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
                let proof = crate::ClaimProofV0::new(proof_data);

                assert!(
                    proof.verify(&root, leaf),
                    "Proof should verify for leaf {} in {}-leaf irregular binary tree",
                    i,
                    size
                );
            }
        }
    }

    #[test]
    fn test_claim_proof_v0_verify_large_tree() {
        // Test: Large tree with 500 leaves
        let campaign = Pubkey::new_unique();
        let claimants: Vec<Pubkey> = (0..500).map(|_| Pubkey::new_unique()).collect();
        let leaves: Vec<ClaimLeaf> = claimants
            .iter()
            .enumerate()
            .map(|(i, &claimant)| ClaimLeaf {
                campaign,
                claimant,
                entitlements: (i + 1) as u64 * 20,
                vault_index: (i % 10) as u8,
            })
            .collect();

        let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
        let root = tree.root().unwrap();

        // Test leaves at various positions
        let test_indices = [0, 1, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 499];
        for &i in &test_indices {
            if i < leaves.len() {
                let leaf = &leaves[i];
                let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
                let proof = crate::ClaimProofV0::new(proof_data);

                assert!(
                    proof.verify(&root, leaf),
                    "Proof should verify for leaf {} in 500-leaf tree",
                    i
                );
            }
        }
    }

    #[test]
    fn test_claim_proof_v0_verify_wrong_root() {
        // Test: Valid proof but wrong root should fail
        let campaign = Pubkey::new_unique();
        let claimants: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();
        let leaves: Vec<ClaimLeaf> = claimants
            .iter()
            .enumerate()
            .map(|(i, &claimant)| ClaimLeaf {
                campaign,
                claimant,
                vault_index: i as u8,
                entitlements: (i + 1) as u64 * 100,
            })
            .collect();

        let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
        let _correct_root = tree.root().unwrap();
        let wrong_root = [0xFF; 32]; // Obviously wrong root

        for leaf in &leaves {
            let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
            let proof = crate::ClaimProofV0::new(proof_data);

            assert!(
                !proof.verify(&wrong_root, leaf),
                "Proof should fail with wrong root"
            );
        }
    }

    #[test]
    fn test_claim_proof_v0_verify_wrong_leaf() {
        // Test: Valid proof and root but wrong leaf should fail
        let campaign = Pubkey::new_unique();
        let claimants: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();
        let leaves: Vec<ClaimLeaf> = claimants
            .iter()
            .enumerate()
            .map(|(i, &claimant)| ClaimLeaf {
                campaign,
                claimant,
                entitlements: (i + 1) as u64 * 100,
                vault_index: i as u8,
            })
            .collect();

        let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
        let root = tree.root().unwrap();

        // Create a fake leaf not in the tree
        let fake_leaf = ClaimLeaf {
            campaign,
            claimant: Pubkey::new_unique(),
            entitlements: 999,
            vault_index: 99,
        };

        for leaf in &leaves {
            let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
            let proof = crate::ClaimProofV0::new(proof_data);

            assert!(
                !proof.verify(&root, &fake_leaf),
                "Proof should fail with wrong leaf data"
            );
        }
    }

    #[test]
    fn test_claim_proof_v0_verify_corrupted_proof() {
        // Test: Corrupted proof data should fail verification
        let campaign = Pubkey::new_unique();
        let claimants: Vec<Pubkey> = (0..4).map(|_| Pubkey::new_unique()).collect();
        let leaves: Vec<ClaimLeaf> = claimants
            .iter()
            .enumerate()
            .map(|(i, &claimant)| ClaimLeaf {
                campaign,
                claimant,
                entitlements: (i + 1) as u64 * 100,
                vault_index: i as u8,
            })
            .collect();

        let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
        let root = tree.root().unwrap();
        let leaf = &leaves[0];

        // Get valid proof and corrupt it
        let mut proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();

        // Corrupt first hash in proof (if proof is not empty)
        if !proof_data.is_empty() {
            proof_data[0] = [0xFF; 32]; // Corrupt the hash

            let corrupted_proof = crate::ClaimProofV0::new(proof_data);
            assert!(
                !corrupted_proof.verify(&root, leaf),
                "Corrupted proof should fail verification"
            );
        }
    }

    #[test]
    fn test_claim_proof_v0_verify_empty_proof_wrong_context() {
        // Test: Empty proof should only work when root == leaf.to_hash()
        let campaign = Pubkey::new_unique();
        let claimant = Pubkey::new_unique();
        let leaf = ClaimLeaf {
            campaign,
            claimant,
            entitlements: 100,
            vault_index: 0,
        };
        let empty_proof = crate::ClaimProofV0::new(vec![]);

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
    fn test_claim_proof_v0_verify_domain_separation() {
        // Test: Ensure domain separation is working (internal nodes vs leaves)
        let campaign = Pubkey::new_unique();
        let claimants: Vec<Pubkey> = (0..4).map(|_| Pubkey::new_unique()).collect();
        let leaves: Vec<ClaimLeaf> = claimants
            .iter()
            .enumerate()
            .map(|(i, &claimant)| ClaimLeaf {
                campaign,
                claimant,
                entitlements: (i + 1) as u64 * 100,
                vault_index: i as u8,
            })
            .collect();

        let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
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
            let proof = crate::ClaimProofV0::new(proof_data);

            assert!(
                proof.verify(&root, leaf),
                "Proof should verify with proper domain separation"
            );
        }
    }

    #[test]
    fn test_claim_proof_v0_verify_proof_size_analysis() {
        // Test: Analyze proof sizes for different tree sizes (binary trees)
        let campaign = Pubkey::new_unique();
        let test_sizes = [1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256];

        for &size in &test_sizes {
            let claimants: Vec<Pubkey> = (0..size).map(|_| Pubkey::new_unique()).collect();
            let leaves: Vec<ClaimLeaf> = claimants
                .iter()
                .enumerate()
                .map(|(i, &claimant)| ClaimLeaf {
                    campaign,
                    claimant,
                    entitlements: (i + 1) as u64 * 50,
                    vault_index: (i % 3) as u8,
                })
                .collect();

            let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
            let root = tree.root().unwrap();

            // Test first leaf's proof
            if !leaves.is_empty() {
                let leaf = &leaves[0];
                let proof_data = tree.proof_for_claimant(&leaf.claimant).unwrap();
                let proof = crate::ClaimProofV0::new(proof_data);

                // Verify the proof works
                assert!(
                    proof.verify(&root, leaf),
                    "Proof should verify for tree size {}",
                    size
                );

                // Calculate expected tree depth for binary tree
                let expected_depth = if size <= 1 {
                    0
                } else {
                    (size as f64).log2().ceil() as usize
                };

                // Log proof characteristics for analysis
                println!(
                    "Tree size: {}, Expected depth: {}, Proof length: {}",
                    size,
                    expected_depth,
                    proof.len()
                );

                // Sanity checks on proof structure
                if size == 1 {
                    assert_eq!(proof.len(), 0, "Single leaf should have empty proof");
                } else {
                    assert!(
                        proof.len() > 0,
                        "Multi-leaf tree should have non-empty proof"
                    );
                    // For binary trees, proof length should be approximately log2(n)
                    assert!(
                        proof.len() <= expected_depth + 1,
                        "Proof length {} should be close to tree depth {}",
                        proof.len(),
                        expected_depth
                    );
                }
            }
        }
    }

    #[test]
    fn test_claim_proof_v0_verify_stress_test() {
        // Test: Stress test with various binary tree configurations
        let campaign = Pubkey::new_unique();
        let configurations = [
            (10, "Small tree"),
            (15, "Irregular small"),
            (16, "Perfect binary (2^4)"),
            (31, "Almost perfect"),
            (32, "Perfect binary (2^5)"),
            (63, "Almost perfect large"),
            (64, "Perfect binary (2^6)"),
            (100, "Large irregular"),
            (128, "Perfect binary (2^7)"),
            (200, "Very large"),
        ];

        for (size, description) in configurations {
            let claimants: Vec<Pubkey> = (0..size).map(|_| Pubkey::new_unique()).collect();
            let leaves: Vec<ClaimLeaf> = claimants
                .iter()
                .enumerate()
                .map(|(i, &claimant)| ClaimLeaf {
                    campaign,
                    claimant,
                    entitlements: (i + 1) as u64 * 37, // Prime number for variety
                    vault_index: (i % 8) as u8,
                })
                .collect();

            let tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();
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
                let proof = crate::ClaimProofV0::new(proof_data);

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
    fn test_claim_proof_v0_verify_cross_tree_isolation() {
        // Test: Proofs from one tree should not verify against another tree's root
        let campaign = Pubkey::new_unique();
        let claimants_a: Vec<Pubkey> = (0..4).map(|_| Pubkey::new_unique()).collect();
        let leaves_tree_a: Vec<ClaimLeaf> = claimants_a
            .iter()
            .enumerate()
            .map(|(i, &claimant)| ClaimLeaf {
                campaign,
                claimant,
                entitlements: (i + 1) as u64 * 100,
                vault_index: i as u8,
            })
            .collect();

        let claimants_b: Vec<Pubkey> = (0..4).map(|_| Pubkey::new_unique()).collect();
        let leaves_tree_b: Vec<ClaimLeaf> = claimants_b
            .iter()
            .enumerate()
            .map(|(i, &claimant)| ClaimLeaf {
                campaign,
                claimant,
                entitlements: (i + 5) as u64 * 200,
                vault_index: (i + 4) as u8,
            })
            .collect();

        let tree_a = ClaimTreeV0::from_leaves(leaves_tree_a.clone()).unwrap();
        let tree_b = ClaimTreeV0::from_leaves(leaves_tree_b.clone()).unwrap();

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
            let proof_a = crate::ClaimProofV0::new(proof_data_a);

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

    #[test]
    fn test_claim_proof_v0_verify_real_world_simulation() {
        // Test: Simulate real-world usage with consistent hashing
        let campaign = Pubkey::new_unique();
        let vault_count = 5;
        let num_claimants = 50;

        // Generate realistic claimant data
        let claimant_entitlements: Vec<(Pubkey, u64)> = (0..num_claimants)
            .map(|i| {
                let claimant = Pubkey::new_unique();
                let entitlements = (i + 1) as u64 * 1000; // Varying entitlements
                (claimant, entitlements)
            })
            .collect();

        // Create tree using the production function
        let tree = create_claim_tree_v0(campaign, &claimant_entitlements, vault_count).unwrap();
        let root = tree.root().unwrap();

        // Test that all claimants can generate valid proofs
        for (claimant, expected_entitlements) in &claimant_entitlements {
            let leaf = tree.leaf_for_claimant(claimant).unwrap();

            // Verify leaf data is correct
            assert_eq!(leaf.claimant, *claimant);
            assert_eq!(leaf.entitlements, *expected_entitlements);
            assert!(leaf.vault_index < vault_count as u8);

            // Generate and verify proof
            let proof_data = tree.proof_for_claimant(claimant).unwrap();
            let proof = crate::ClaimProofV0::new(proof_data);

            assert!(
                proof.verify(&root, leaf),
                "Real-world proof should verify for claimant {:?}",
                claimant
            );
        }
    }
}

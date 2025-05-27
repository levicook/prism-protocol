use crate::merkle_leaf::{hash_claim_leaf, ClaimLeaf};
use crate::merkle_tree::hasher::PrismHasher;
use anchor_lang::prelude::*;
use rs_merkle::MerkleTree;
use std::collections::HashMap;

/// Result of building a merkle tree from claim leaves
#[derive(Clone)]
pub struct ClaimMerkleTree {
    /// The underlying merkle tree
    pub tree: MerkleTree<PrismHasher>,
    /// Mapping from claimant pubkey to their leaf index in the tree
    pub claimant_to_index: HashMap<Pubkey, usize>,
    /// The original leaves used to build the tree
    pub leaves: Vec<ClaimLeaf>,
}

impl ClaimMerkleTree {
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
        let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|leaf| hash_claim_leaf(leaf)).collect();

        // Build the merkle tree
        let tree = MerkleTree::<PrismHasher>::from_leaves(&leaf_hashes);

        Ok(ClaimMerkleTree {
            tree,
            claimant_to_index,
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
            .claimant_to_index
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
            .claimant_to_index
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
            .claimant_to_index
            .get(claimant)
            .ok_or(ErrorCode::ClaimantNotFound)?;
        let leaf = self.leaf_for_claimant(claimant)?;
        let leaf_hash = hash_claim_leaf(leaf);

        let merkle_proof = rs_merkle::MerkleProof::<PrismHasher>::new(proof.to_vec());

        Ok(merkle_proof.verify(root, &[*index], &[leaf_hash], self.leaves.len()))
    }
}

/// Helper function to create a simple test merkle tree with known claimants
#[cfg(feature = "testing")]
pub fn create_test_merkle_tree(
    claimants: &[Pubkey],
    vaults: &[Pubkey],
    entitlements_per_claimant: u64,
) -> Result<ClaimMerkleTree> {
    require!(!claimants.is_empty(), ErrorCode::InvalidInput);
    require!(!vaults.is_empty(), ErrorCode::InvalidInput);

    let leaves: Vec<ClaimLeaf> = claimants
        .iter()
        .enumerate()
        .map(|(i, &claimant)| {
            // Simple round-robin assignment to vaults
            let vault_index = i % vaults.len();
            ClaimLeaf {
                claimant,
                assigned_vault: vaults[vault_index],
                entitlements: entitlements_per_claimant,
            }
        })
        .collect();

    ClaimMerkleTree::from_leaves(leaves)
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
        let claimants = [
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        let vault = Pubkey::new_unique();

        let leaves = vec![
            ClaimLeaf {
                claimant: claimants[0],
                assigned_vault: vault,
                entitlements: 100,
            },
            ClaimLeaf {
                claimant: claimants[1],
                assigned_vault: vault,
                entitlements: 200,
            },
            ClaimLeaf {
                claimant: claimants[2],
                assigned_vault: vault,
                entitlements: 300,
            },
        ];

        let merkle_tree = ClaimMerkleTree::from_leaves(leaves.clone()).unwrap();

        // Verify root exists
        assert!(merkle_tree.root().is_some());

        // Verify all claimants are mapped
        for (i, claimant) in claimants.iter().enumerate() {
            assert_eq!(merkle_tree.claimant_to_index[claimant], i);
        }

        // Verify leaves are stored correctly
        assert_eq!(merkle_tree.leaves, leaves);
    }

    #[test]
    fn test_generate_and_verify_proof() {
        let claimants = [Pubkey::new_unique(), Pubkey::new_unique()];
        let vault = Pubkey::new_unique();

        let leaves = vec![
            ClaimLeaf {
                claimant: claimants[0],
                assigned_vault: vault,
                entitlements: 100,
            },
            ClaimLeaf {
                claimant: claimants[1],
                assigned_vault: vault,
                entitlements: 200,
            },
        ];

        let merkle_tree = ClaimMerkleTree::from_leaves(leaves).unwrap();

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
        let claimant = Pubkey::new_unique();
        let vault = Pubkey::new_unique();

        let leaves = vec![
            ClaimLeaf {
                claimant,
                assigned_vault: vault,
                entitlements: 100,
            },
            ClaimLeaf {
                claimant, // Duplicate!
                assigned_vault: vault,
                entitlements: 200,
            },
        ];

        let result = ClaimMerkleTree::from_leaves(leaves);
        assert!(result.is_err());
    }

    #[cfg(feature = "testing")]
    #[test]
    fn test_create_test_merkle_tree() {
        let claimants = [
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        let vaults = [Pubkey::new_unique(), Pubkey::new_unique()];
        let entitlements = 1000;

        let merkle_tree = create_test_merkle_tree(&claimants, &vaults, entitlements).unwrap();

        // Verify tree was created successfully
        assert!(merkle_tree.root().is_some());
        assert_eq!(merkle_tree.leaves.len(), claimants.len());

        // Verify round-robin vault assignment
        for (i, claimant) in claimants.iter().enumerate() {
            let leaf = merkle_tree.leaf_for_claimant(claimant).unwrap();
            let expected_vault = vaults[i % vaults.len()];
            assert_eq!(leaf.assigned_vault, expected_vault);
            assert_eq!(leaf.entitlements, entitlements);
        }
    }
}

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
    claimant_entitlements: &[(Pubkey, u64)],
    vault_count: usize,
) -> Result<ClaimTreeV0> {
    require!(!claimant_entitlements.is_empty(), ErrorCode::InvalidInput);
    require!(vault_count > 0, ErrorCode::InvalidInput);

    let leaves: Vec<ClaimLeaf> = claimant_entitlements
        .iter()
        .map(|(claimant, entitlements)| {
            // Consistent hashing: hash the claimant pubkey to determine vault assignment
            let vault_index = consistent_hash_vault_assignment(claimant, vault_count);

            ClaimLeaf {
                claimant: *claimant,
                assigned_vault_index: vault_index as u8,
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
    pub claimant_to_index: HashMap<Pubkey, usize>,
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
        let claimants = [
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];

        let leaves = vec![
            ClaimLeaf {
                claimant: claimants[0],
                assigned_vault_index: 0,
                entitlements: 100,
            },
            ClaimLeaf {
                claimant: claimants[1],
                assigned_vault_index: 0,
                entitlements: 200,
            },
            ClaimLeaf {
                claimant: claimants[2],
                assigned_vault_index: 1,
                entitlements: 300,
            },
        ];

        let merkle_tree = ClaimTreeV0::from_leaves(leaves.clone()).unwrap();

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

        let leaves = vec![
            ClaimLeaf {
                claimant: claimants[0],
                assigned_vault_index: 0,
                entitlements: 100,
            },
            ClaimLeaf {
                claimant: claimants[1],
                assigned_vault_index: 1,
                entitlements: 200,
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
        let claimant = Pubkey::new_unique();

        let leaves = vec![
            ClaimLeaf {
                claimant,
                assigned_vault_index: 0,
                entitlements: 100,
            },
            ClaimLeaf {
                claimant, // Duplicate!
                assigned_vault_index: 1,
                entitlements: 200,
            },
        ];

        let result = ClaimTreeV0::from_leaves(leaves);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_merkle_tree_consistent_hashing() {
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

        let merkle_tree = create_claim_tree_v0(&claimant_entitlements, vault_count).unwrap();

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
            assert_eq!(leaf.assigned_vault_index as usize, vault_index);
        }

        // Test determinism: creating the tree again should produce identical assignments
        let merkle_tree2 = create_claim_tree_v0(&claimant_entitlements, vault_count).unwrap();
        for (leaf1, leaf2) in merkle_tree.leaves.iter().zip(merkle_tree2.leaves.iter()) {
            assert_eq!(leaf1.claimant, leaf2.claimant);
            assert_eq!(
                leaf1.assigned_vault_index as usize,
                leaf2.assigned_vault_index as usize
            );
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
            let vault_index = crate::consistent_hash_vault_assignment(&claimant, vault_count);
            vault_counts[vault_index] += 1;
        }

        // Each vault should get roughly 200 claimants (1000/5)
        // Allow some variance - each should be between 150-250
        for count in vault_counts {
            assert!(count >= 150 && count <= 250, "Vault count: {}", count);
        }
    }
}

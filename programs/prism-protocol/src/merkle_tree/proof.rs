use crate::merkle_leaf::{hash_claim_leaf, ClaimLeaf};
use crate::merkle_tree::hasher::SplHasher;
use anchor_lang::prelude::*;
use rs_merkle::MerkleProof;

/// Verify a merkle proof against a root and leaf data
/// This is a convenience wrapper around the on-chain verification logic
pub fn verify_claim_proof(
    proof: &[[u8; 32]],
    root: &[u8; 32],
    leaf: &ClaimLeaf,
    leaf_index: usize,
    total_leaves: usize,
) -> bool {
    let leaf_hash = hash_claim_leaf(leaf);
    let merkle_proof = MerkleProof::<SplHasher>::new(proof.to_vec());
    
    merkle_proof.verify(
        *root,
        &[leaf_index],
        &[leaf_hash],
        total_leaves,
    )
}

/// Generate a proof for a specific leaf in a tree
pub fn generate_proof_for_leaf(
    leaves: &[ClaimLeaf],
    target_leaf: &ClaimLeaf,
) -> Result<Vec<[u8; 32]>> {
    // Find the index of the target leaf
    let leaf_index = leaves
        .iter()
        .position(|leaf| {
            leaf.claimant == target_leaf.claimant
                && leaf.assigned_vault == target_leaf.assigned_vault
                && leaf.entitlements == target_leaf.entitlements
        })
        .ok_or_else(|| error!(ErrorCode::LeafNotFound))?;

    // Hash all leaves
    let leaf_hashes: Vec<[u8; 32]> = leaves
        .iter()
        .map(|leaf| hash_claim_leaf(leaf))
        .collect();

    // Build tree and generate proof
    let tree = rs_merkle::MerkleTree::<SplHasher>::from_leaves(&leaf_hashes);
    let proof = tree.proof(&[leaf_index]);
    
    Ok(proof.proof_hashes().to_vec())
}

/// Batch verify multiple proofs against the same root
pub fn batch_verify_proofs(
    proofs: &[(&[[u8; 32]], &ClaimLeaf, usize)], // (proof, leaf, index)
    root: &[u8; 32],
    total_leaves: usize,
) -> Vec<bool> {
    proofs
        .iter()
        .map(|(proof, leaf, index)| {
            verify_claim_proof(proof, root, leaf, *index, total_leaves)
        })
        .collect()
}

/// Extract the root from a proof and leaf data (useful for testing)
pub fn extract_root_from_proof(
    proof: &[[u8; 32]],
    leaf: &ClaimLeaf,
    leaf_index: usize,
    total_leaves: usize,
) -> Option<[u8; 32]> {
    let leaf_hash = hash_claim_leaf(leaf);
    let merkle_proof = MerkleProof::<SplHasher>::new(proof.to_vec());
    
    merkle_proof.root(&[leaf_index], &[leaf_hash], total_leaves).ok()
}

#[error_code]
pub enum ErrorCode {
    #[msg("Leaf not found in the provided leaves")]
    LeafNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_leaves() -> Vec<ClaimLeaf> {
        let claimants = [Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::new_unique()];
        let vault = Pubkey::new_unique();
        
        vec![
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
        ]
    }

    #[test]
    fn test_generate_and_verify_proof() {
        let leaves = create_test_leaves();
        let target_leaf = &leaves[1]; // Middle leaf

        // Generate proof
        let proof = generate_proof_for_leaf(&leaves, target_leaf).unwrap();

        // Build tree to get root
        let leaf_hashes: Vec<[u8; 32]> = leaves
            .iter()
            .map(|leaf| hash_claim_leaf(leaf))
            .collect();
        let tree = rs_merkle::MerkleTree::<SplHasher>::from_leaves(&leaf_hashes);
        let root = tree.root().unwrap();

        // Verify proof
        let is_valid = verify_claim_proof(&proof, &root, target_leaf, 1, leaves.len());
        assert!(is_valid, "Generated proof should be valid");

        // Verify proof fails for wrong leaf
        let wrong_leaf = &leaves[0];
        let is_valid = verify_claim_proof(&proof, &root, wrong_leaf, 1, leaves.len());
        assert!(!is_valid, "Proof should be invalid for wrong leaf");
    }

    #[test]
    fn test_extract_root_from_proof() {
        let leaves = create_test_leaves();
        let target_leaf = &leaves[0];

        // Generate proof
        let proof = generate_proof_for_leaf(&leaves, target_leaf).unwrap();

        // Build tree to get expected root
        let leaf_hashes: Vec<[u8; 32]> = leaves
            .iter()
            .map(|leaf| hash_claim_leaf(leaf))
            .collect();
        let tree = rs_merkle::MerkleTree::<SplHasher>::from_leaves(&leaf_hashes);
        let expected_root = tree.root().unwrap();

        // Extract root from proof
        let extracted_root = extract_root_from_proof(&proof, target_leaf, 0, leaves.len()).unwrap();

        assert_eq!(extracted_root, expected_root, "Extracted root should match tree root");
    }

    #[test]
    fn test_batch_verify_proofs() {
        let leaves = create_test_leaves();

        // Generate proofs for all leaves
        let proofs: Vec<Vec<[u8; 32]>> = leaves
            .iter()
            .map(|leaf| generate_proof_for_leaf(&leaves, leaf).unwrap())
            .collect();

        // Build tree to get root
        let leaf_hashes: Vec<[u8; 32]> = leaves
            .iter()
            .map(|leaf| hash_claim_leaf(leaf))
            .collect();
        let tree = rs_merkle::MerkleTree::<SplHasher>::from_leaves(&leaf_hashes);
        let root = tree.root().unwrap();

        // Prepare batch verification data
        let batch_data: Vec<(&[[u8; 32]], &ClaimLeaf, usize)> = proofs
            .iter()
            .zip(leaves.iter())
            .enumerate()
            .map(|(i, (proof, leaf))| (proof.as_slice(), leaf, i))
            .collect();

        // Batch verify
        let results = batch_verify_proofs(&batch_data, &root, leaves.len());

        // All proofs should be valid
        assert!(results.iter().all(|&result| result), "All proofs should be valid");
    }

    #[test]
    fn test_leaf_not_found_error() {
        let leaves = create_test_leaves();
        let non_existent_leaf = ClaimLeaf {
            claimant: Pubkey::new_unique(),
            assigned_vault: Pubkey::new_unique(),
            entitlements: 999,
        };

        let result = generate_proof_for_leaf(&leaves, &non_existent_leaf);
        assert!(result.is_err(), "Should return error for non-existent leaf");
    }
} 
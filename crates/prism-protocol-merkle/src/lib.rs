pub mod claim_tree_v0;
pub mod claim_tree_v1;
pub mod hasher_v0;
pub mod hasher_v1;
pub mod proof;

pub use claim_tree_v0::{create_claim_tree_v0, ClaimTreeV0};
pub use claim_tree_v1::{create_claim_tree_v1, ClaimTreeV1};
pub use hasher_v0::ClaimHasherV0;
pub use hasher_v1::ClaimHasherV1;
pub use proof::{
    batch_verify_proofs, extract_root_from_proof, generate_proof_for_leaf, verify_claim_proof,
};

// Re-export merkle leaf and proof types from prism protocol
pub use prism_protocol::{ClaimLeaf, ClaimProofV0, ClaimProofV1};

/// Shared constants for merkle tree implementations
pub mod claim_tree_constants {
    /// Number of children per internal node in the 256-ary merkle tree
    pub const BRANCHING_FACTOR: usize = 256;

    /// Domain separation prefix for leaf nodes
    pub const LEAF_PREFIX: u8 = prism_protocol::claim_tree_constants::LEAF_PREFIX;

    /// Domain separation prefix for internal nodes
    pub const INTERNAL_PREFIX: u8 = prism_protocol::claim_tree_constants::INTERNAL_PREFIX;
}

/// Performs consistent hashing to assign a claimant to a vault index.
///
/// ## ⚠️ CRITICAL: This function must remain stable across all tree versions (V0, V1, etc.)
///
/// This function implements deterministic vault assignment using SHA256 hashing
/// of the claimant's pubkey. The algorithm must never change to ensure:
/// - **Cross-version compatibility**: V0 and V1 trees assign the same claimant to the same vault
/// - **Upgrade compatibility**: Existing deployments can safely upgrade tree versions
/// - **Deterministic behavior**: Same input always produces same output
///
/// ## Algorithm
/// 1. SHA256 hash the claimant's pubkey bytes
/// 2. Take the first 8 bytes as little-endian u64  
/// 3. Modulo by vault_count to get vault index
///
/// ## Usage
/// Used by both `create_merkle_tree_v0` and `create_merkle_tree_v1` to ensure
/// identical vault assignments regardless of tree structure.
pub(crate) fn consistent_hash_vault_assignment(
    claimant: &anchor_lang::prelude::Pubkey,
    vault_count: u8,
) -> u8 {
    use anchor_lang::solana_program::hash::Hasher;

    let mut hasher = Hasher::default();
    hasher.hash(claimant.as_ref());
    let hash_bytes = hasher.result().to_bytes();

    // Convert first 8 bytes to u64 (little-endian)
    let hash_u64 = u64::from_le_bytes([
        hash_bytes[0],
        hash_bytes[1],
        hash_bytes[2],
        hash_bytes[3],
        hash_bytes[4],
        hash_bytes[5],
        hash_bytes[6],
        hash_bytes[7],
    ]);

    (hash_u64 as u8) % vault_count
}

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::Pubkey;

    use super::*;

    #[test]
    fn test_cross_version_vault_assignment_compatibility() {
        let campaign = Pubkey::new_unique();
        // 🔒 CRITICAL TEST: Ensure V0 and V1 trees assign claimants to identical vaults
        let claimants = [
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        let vault_count = 3;

        let claimant_entitlements: Vec<(Pubkey, u64)> = claimants
            .iter()
            .enumerate()
            .map(|(i, &claimant)| (claimant, (i + 1) as u64 * 100))
            .collect();

        // Create both V0 and V1 trees
        let tree_v0 = create_claim_tree_v0(campaign, &claimant_entitlements, vault_count).unwrap();
        let tree_v1 = create_claim_tree_v1(campaign, &claimant_entitlements, vault_count).unwrap();

        // Verify that every claimant gets assigned to the same vault in both versions
        for (claimant, _entitlements) in claimant_entitlements.iter() {
            let v0_leaf = tree_v0.leaf_for_claimant(claimant).unwrap();
            let v1_leaf = tree_v1.leaf_for_claimant(claimant).unwrap();

            assert_eq!(
                v0_leaf.vault_index, v1_leaf.vault_index,
                "Claimant {:?} assigned to different vaults: V0={}, V1={}",
                claimant, v0_leaf.vault_index, v1_leaf.vault_index
            );

            // Also verify campaign, claimant, and entitlements are identical
            assert_eq!(v0_leaf.campaign, v1_leaf.campaign);
            assert_eq!(v0_leaf.claimant, v1_leaf.claimant);
            assert_eq!(v0_leaf.entitlements, v1_leaf.entitlements);
        }

        println!("✅ Cross-version compatibility verified: V0 and V1 trees assign identical vault assignments");
    }
}

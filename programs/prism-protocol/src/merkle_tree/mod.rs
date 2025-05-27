#![cfg(feature = "merkle-tree")]

pub mod builder;
pub mod hasher;
pub mod proof;

pub use builder::ClaimMerkleTree;
pub use hasher::PrismHasher;
pub use proof::{
    batch_verify_proofs, extract_root_from_proof, generate_proof_for_leaf, verify_claim_proof,
};

#[cfg(feature = "testing")]
pub use builder::create_test_merkle_tree;

// Re-export key types from rs-merkle for convenience
pub use rs_merkle::{MerkleProof, MerkleTree};

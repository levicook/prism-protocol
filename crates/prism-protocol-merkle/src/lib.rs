pub mod builder;
pub mod hasher;
pub mod proof;

pub use builder::{create_merkle_tree, ClaimMerkleTree};
pub use hasher::PrismHasher;
pub use proof::{
    batch_verify_proofs, extract_root_from_proof, generate_proof_for_leaf, verify_claim_proof,
};

// Re-export merkle leaf from prism protocol
pub use prism_protocol::{hash_claim_leaf, ClaimLeaf};

// Re-export key types from rs-merkle for convenience
pub use rs_merkle::{MerkleProof, MerkleTree};

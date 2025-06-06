use anchor_lang::prelude::*;
use prism_protocol::{ClaimProofV0, ClaimProofV1};
use prism_protocol_entities::compiled_proofs;
use solana_sdk::pubkey::Pubkey;

pub trait CompiledProofExt {
    fn cohort_address(&self) -> Pubkey;
    fn merkle_proof_v0(&self) -> Vec<[u8; 32]>;
    fn merkle_proof_v1(&self) -> Vec<Vec<[u8; 32]>>;
}

impl CompiledProofExt for compiled_proofs::Model {
    fn cohort_address(&self) -> Pubkey {
        let result = self.cohort_address.parse::<Pubkey>();
        debug_assert!(
            result.is_ok(),
            "Invalid cohort address {}",
            self.cohort_address
        );
        result.unwrap()
    }

    fn merkle_proof_v0(&self) -> Vec<[u8; 32]> {
        let merkle_proof_bytes = hex::decode(self.merkle_proof.as_bytes());
        debug_assert!(
            merkle_proof_bytes.is_ok(),
            "Invalid hex in merkle proof: {}",
            self.merkle_proof
        );

        let claim_proof_v0 = ClaimProofV0::try_from_slice(&merkle_proof_bytes.unwrap());
        debug_assert!(
            claim_proof_v0.is_ok(),
            "Failed to deserialize ClaimProofV0 from: {}",
            self.merkle_proof
        );

        claim_proof_v0.unwrap().into_inner()
    }

    fn merkle_proof_v1(&self) -> Vec<Vec<[u8; 32]>> {
        let merkle_proof_bytes = hex::decode(self.merkle_proof.as_bytes());
        debug_assert!(
            merkle_proof_bytes.is_ok(),
            "Invalid hex in merkle proof: {}",
            self.merkle_proof
        );

        let claim_proof_v1 = ClaimProofV1::try_from_slice(&merkle_proof_bytes.unwrap());
        debug_assert!(
            claim_proof_v1.is_ok(),
            "Failed to deserialize ClaimProofV1 from: {}",
            self.merkle_proof
        );

        claim_proof_v1.unwrap().into_inner()
    }
}

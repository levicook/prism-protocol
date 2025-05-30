use anchor_lang::prelude::*;

use prism_protocol::{
    CAMPAIGN_V0_SEED_PREFIX, CLAIM_RECEIPT_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX,
    ID as PRISM_PROGRAM_ID, VAULT_SEED_PREFIX,
};

pub struct AddressFinder {
    pub program_id: Pubkey,
}

impl AddressFinder {
    pub fn new(program_id: Pubkey) -> Self {
        Self { program_id }
    }

    pub fn program_id(&self) -> &Pubkey {
        &self.program_id
    }

    pub fn find_campaign_v0_address(
        &self,
        authority: &Pubkey,
        fingerprint: &[u8; 32],
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                CAMPAIGN_V0_SEED_PREFIX,
                authority.as_ref(),
                fingerprint.as_ref(),
            ],
            &self.program_id,
        )
    }

    pub fn find_cohort_v0_address(
        &self,
        campaign_address: &Pubkey,
        cohort_merkle_root: &[u8; 32],
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                COHORT_V0_SEED_PREFIX,
                campaign_address.as_ref(),
                cohort_merkle_root.as_ref(),
            ],
            &self.program_id,
        )
    }

    pub fn find_claim_receipt_v0_address(
        &self,
        cohort_address: &Pubkey,
        claimant_address: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                CLAIM_RECEIPT_V0_SEED_PREFIX,
                cohort_address.as_ref(),
                claimant_address.as_ref(),
            ],
            &self.program_id,
        )
    }

    pub fn find_vault_v0_address(&self, cohort_address: &Pubkey, vault_index: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[VAULT_SEED_PREFIX, cohort_address.as_ref(), &[vault_index]],
            &self.program_id,
        )
    }
}

impl Default for AddressFinder {
    fn default() -> Self {
        Self::new(PRISM_PROGRAM_ID)
    }
}

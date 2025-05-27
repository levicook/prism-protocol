use anchor_lang::prelude::*;

use prism_protocol::{
    CAMPAIGN_V0_SEED_PREFIX, CLAIM_RECEIPT_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX,
    ID as PRISM_PROGRAM_ID,
};

pub fn find_campaign_address(authority: &Pubkey, fingerprint: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            CAMPAIGN_V0_SEED_PREFIX,
            authority.as_ref(),
            fingerprint.as_ref(),
        ],
        &PRISM_PROGRAM_ID,
    )
}

pub fn find_cohort_v0_address(
    campaign_address: &Pubkey,
    cohort_merkle_root: &[u8; 32],
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            COHORT_V0_SEED_PREFIX,
            campaign_address.as_ref(),
            cohort_merkle_root.as_ref(),
        ],
        &PRISM_PROGRAM_ID,
    )
}

pub fn find_claim_receipt_v0_address(
    cohort_address: &Pubkey,
    claimant_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            CLAIM_RECEIPT_V0_SEED_PREFIX,
            cohort_address.as_ref(),
            claimant_address.as_ref(),
        ],
        &PRISM_PROGRAM_ID,
    )
}

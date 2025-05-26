use anchor_lang::prelude::*;

use crate::{CAMPAIGN_V0_SEED_PREFIX, ID as PRISM_PROGRAM_ID};

pub fn find_campaign_address(authority: &Pubkey, fingerprint: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[CAMPAIGN_V0_SEED_PREFIX, authority.as_ref(), fingerprint.as_ref()],
        &PRISM_PROGRAM_ID,
    )
}

pub fn find_cohort_v0_address(
    campaign_address: &Pubkey,
    cohort_merkle_root: &[u8; 32],
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"cohort".as_ref(),
            campaign_address.as_ref(),
            cohort_merkle_root.as_ref(),
        ],
        &PRISM_PROGRAM_ID,
    )
}

pub fn find_claim_receipt_address(
    cohort_address: &Pubkey,
    claimant_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"claim_receipt".as_ref(),
            cohort_address.as_ref(),
            claimant_address.as_ref(),
        ],
        &PRISM_PROGRAM_ID,
    )
}

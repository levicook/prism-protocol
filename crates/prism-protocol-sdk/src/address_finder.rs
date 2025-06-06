use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    system_program::ID as SYSTEM_PROGRAM_ID, sysvar::rent::ID as RENT_ID,
};
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::{
    associated_token::ID as ASSOCIATED_TOKEN_PROGRAM_ID, token::ID as TOKEN_PROGRAM_ID,
};
use prism_protocol::{
    CLAIM_RECEIPT_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX, ID as PRISM_PROGRAM_ID, VAULT_SEED_PREFIX,
};

#[derive(Clone)]
pub struct AddressFinder {
    pub admin: Pubkey,
    pub campaign: Pubkey,
    pub mint: Pubkey,

    pub prism_program_id: Pubkey,
    pub associated_token_program_id: Pubkey,
    pub rent_id: Pubkey,
    pub system_program_id: Pubkey,
    pub token_program_id: Pubkey,
}

impl AddressFinder {
    pub fn new(admin: Pubkey, campaign: Pubkey, mint: Pubkey) -> Self {
        Self::new_with_program_ids(
            admin,
            campaign,
            mint,
            PRISM_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID,
            RENT_ID,
            SYSTEM_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
        )
    }

    pub fn new_with_program_ids(
        admin: Pubkey,
        campaign: Pubkey,
        mint: Pubkey,
        prism_program_id: Pubkey,
        associated_token_program_id: Pubkey,
        rent_id: Pubkey,
        system_program_id: Pubkey,
        token_program_id: Pubkey,
    ) -> Self {
        Self {
            admin,
            campaign,
            mint,
            prism_program_id,
            associated_token_program_id,
            rent_id,
            system_program_id,
            token_program_id,
        }
    }

    pub fn find_cohort_v0_address(&self, merkle_root: &[u8; 32]) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                COHORT_V0_SEED_PREFIX,
                self.campaign.as_ref(),
                merkle_root.as_ref(),
            ],
            &self.prism_program_id,
        )
    }

    pub fn find_vault_v0_address(&self, cohort: &Pubkey, vault_index: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[VAULT_SEED_PREFIX, cohort.as_ref(), &[vault_index]],
            &self.prism_program_id,
        )
    }

    pub fn find_claim_receipt_v0_address(
        &self,
        cohort: &Pubkey,
        claimant: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                CLAIM_RECEIPT_V0_SEED_PREFIX,
                cohort.as_ref(),
                claimant.as_ref(),
            ],
            &self.prism_program_id,
        )
    }

    pub fn find_claimant_token_account(&self, claimant: &Pubkey) -> Pubkey {
        get_associated_token_address(claimant, &self.mint)
    }
}

pub mod constants;
pub mod error;
pub mod instructions;
pub mod merkle_leaf;
pub mod state;

pub use constants::{
    CAMPAIGN_V0_SEED_PREFIX, CLAIM_RECEIPT_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX, VAULT_SEED_PREFIX,
};
pub use instructions::*;
pub use merkle_leaf::*;
pub use state::*;

use anchor_lang::prelude::*;

declare_id!("PrsmV9Kh8HcJjPSShFidZFJrbWM5NWQ98ST8M2BNdAw");

#[program]
pub mod prism_protocol {
    use super::instructions;
    use super::*;

    // admin
    pub fn initialize_campaign_v0(
        ctx: Context<InitializeCampaignV0>,
        campaign_fingerprint: [u8; 32],
        mint: Pubkey,
        expected_cohort_count: u8,
    ) -> Result<()> {
        instructions::handle_initialize_campaign_v0(
            ctx,
            campaign_fingerprint,
            mint,
            expected_cohort_count,
        )
    }

    // admin
    pub fn activate_campaign_v0(
        ctx: Context<ActivateCampaignV0>,
        campaign_fingerprint: [u8; 32],
        final_db_ipfs_hash: [u8; 32],
        go_live_slot: u64,
    ) -> Result<()> {
        instructions::handle_activate_campaign_v0(
            ctx,
            campaign_fingerprint,
            final_db_ipfs_hash,
            go_live_slot,
        )
    }

    // admin
    pub fn initialize_cohort_v0(
        ctx: Context<InitializeCohortV0>,
        campaign_fingerprint: [u8; 32],
        merkle_root: [u8; 32],
        amount_per_entitlement: u64,
        expected_vault_count: u8,
    ) -> Result<()> {
        instructions::handle_initialize_cohort_v0(
            ctx,
            campaign_fingerprint,
            merkle_root,
            amount_per_entitlement,
            expected_vault_count,
        )
    }

    // admin
    pub fn activate_cohort_v0(
        ctx: Context<ActivateCohortV0>,
        campaign_fingerprint: [u8; 32],
        cohort_merkle_root: [u8; 32],
    ) -> Result<()> {
        instructions::handle_activate_cohort_v0(ctx, campaign_fingerprint, cohort_merkle_root)
    }

    // admin
    pub fn initialize_vault_v0(
        ctx: Context<InitializeVaultV0>,
        campaign_fingerprint: [u8; 32],
        cohort_merkle_root: [u8; 32],
        vault_index: u8,
    ) -> Result<()> {
        instructions::handle_initialize_vault_v0(
            ctx,
            campaign_fingerprint,
            cohort_merkle_root,
            vault_index,
        )
    }

    // admin
    pub fn activate_vault_v0(
        ctx: Context<ActivateVaultV0>,
        campaign_fingerprint: [u8; 32],
        cohort_merkle_root: [u8; 32],
        vault_index: u8,
        expected_balance: u64,
    ) -> Result<()> {
        instructions::handle_activate_vault_v0(
            ctx,
            campaign_fingerprint,
            cohort_merkle_root,
            vault_index,
            expected_balance,
        )
    }

    // claimant
    pub fn claim_tokens_v0(
        ctx: Context<ClaimTokensV0>,
        campaign_fingerprint: [u8; 32],
        cohort_merkle_root: [u8; 32],
        merkle_proof: Vec<[u8; 32]>,
        assigned_vault_index: u8,
        entitlements: u64,
    ) -> Result<()> {
        instructions::handle_claim_tokens_v0(
            ctx,
            campaign_fingerprint,
            cohort_merkle_root,
            merkle_proof,
            assigned_vault_index,
            entitlements,
        )
    }

    // admin
    pub fn reclaim_tokens(
        ctx: Context<ReclaimTokens>,
        campaign_fingerprint: [u8; 32],
        cohort_merkle_root_arg: [u8; 32],
        vault_index: u8,
    ) -> Result<()> {
        instructions::handle_reclaim_tokens(
            ctx,
            campaign_fingerprint,
            cohort_merkle_root_arg,
            vault_index,
        )
    }
}

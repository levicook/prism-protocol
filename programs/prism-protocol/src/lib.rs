pub mod claim_leaf;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod proofs;
pub mod state;

pub use claim_leaf::*;
pub use constants::{
    CAMPAIGN_V0_SEED_PREFIX, CLAIM_RECEIPT_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX, VAULT_SEED_PREFIX,
};
pub use instructions::*;
pub use proofs::*;
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
    pub fn make_campaign_unstoppable_v0(
        ctx: Context<MakeCampaignUnstoppableV0>,
        campaign_fingerprint: [u8; 32],
    ) -> Result<()> {
        instructions::handle_make_campaign_unstoppable_v0(ctx, campaign_fingerprint)
    }

    // admin
    pub fn pause_campaign_v0(
        ctx: Context<PauseCampaignV0>,
        campaign_fingerprint: [u8; 32],
    ) -> Result<()> {
        instructions::handle_pause_campaign_v0(ctx, campaign_fingerprint)
    }

    // admin
    pub fn resume_campaign_v0(
        ctx: Context<ResumeCampaignV0>,
        campaign_fingerprint: [u8; 32],
    ) -> Result<()> {
        instructions::handle_resume_campaign_v0(ctx, campaign_fingerprint)
    }

    // admin
    pub fn permanently_halt_campaign_v0(
        ctx: Context<PermanentlyHaltCampaignV0>,
        campaign_fingerprint: [u8; 32],
    ) -> Result<()> {
        instructions::handle_permanently_halt_campaign_v0(ctx, campaign_fingerprint)
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

    // claimant
    pub fn claim_tokens_v1(
        ctx: Context<ClaimTokensV1>,
        campaign_fingerprint: [u8; 32],
        cohort_merkle_root: [u8; 32],
        merkle_proof: Vec<Vec<[u8; 32]>>,
        assigned_vault_index: u8,
        entitlements: u64,
    ) -> Result<()> {
        instructions::handle_claim_tokens_v1(
            ctx,
            campaign_fingerprint,
            cohort_merkle_root,
            merkle_proof,
            assigned_vault_index,
            entitlements,
        )
    }

    // admin
    pub fn reclaim_tokens_v0(
        ctx: Context<ReclaimTokensV0>,
        campaign_fingerprint: [u8; 32],
        cohort_merkle_root_arg: [u8; 32],
        vault_index: u8,
    ) -> Result<()> {
        instructions::handle_reclaim_tokens_v0(
            ctx,
            campaign_fingerprint,
            cohort_merkle_root_arg,
            vault_index,
        )
    }
}

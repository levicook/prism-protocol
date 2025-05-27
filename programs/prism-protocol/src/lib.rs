pub mod constants;
pub mod error;
pub mod instructions;
pub mod merkle_leaf;
pub mod state;

pub use constants::*;
pub use instructions::*;
pub use merkle_leaf::*;
pub use state::*;

use anchor_lang::prelude::*;

declare_id!("3hLSRKWiLGPgfvJJWe6WP9kCNQB5FVZu3Y4Awp1sMEbJ");

#[program]
pub mod prism_protocol {
    use super::instructions;
    use super::*;

    // admin
    pub fn initialize_campaign_v0(
        ctx: Context<InitializeCampaignV0>,
        campaign_fingerprint: [u8; 32],
        mint: Pubkey,
    ) -> Result<()> {
        instructions::handle_initialize_campaign_v0(ctx, campaign_fingerprint, mint)
    }

    // admin
    pub fn initialize_cohort_v0(
        ctx: Context<InitializeCohortV0>,
        campaign_fingerprint: [u8; 32],
        merkle_root: [u8; 32],
        amount_per_entitlement: u64,
        vaults: Vec<Pubkey>,
    ) -> Result<()> {
        instructions::handle_initialize_cohort_v0(
            ctx,
            campaign_fingerprint,
            merkle_root,
            amount_per_entitlement,
            vaults,
        )
    }

    // admin
    pub fn set_campaign_active_status(
        ctx: Context<SetCampaignActiveStatus>,
        campaign_fingerprint: [u8; 32],
        is_active: bool,
    ) -> Result<()> {
        instructions::handle_set_campaign_active_status(ctx, campaign_fingerprint, is_active)
    }

    // admin
    pub fn reclaim_tokens(
        ctx: Context<ReclaimTokens>,
        campaign_fingerprint: [u8; 32],
        cohort_merkle_root_arg: [u8; 32],
    ) -> Result<()> {
        instructions::handle_reclaim_tokens(ctx, campaign_fingerprint, cohort_merkle_root_arg)
    }

    // claimant
    pub fn claim_tokens_v0(
        ctx: Context<ClaimTokensV0>,
        campaign_fingerprint: [u8; 32],
        cohort_merkle_root_arg: [u8; 32],
        merkle_proof: Vec<[u8; 32]>,
        assigned_vault_from_leaf: Pubkey,
        entitlements_from_leaf: u64,
    ) -> Result<()> {
        instructions::handle_claim_tokens_v0(
            ctx,
            campaign_fingerprint,
            cohort_merkle_root_arg,
            merkle_proof,
            assigned_vault_from_leaf,
            entitlements_from_leaf,
        )
    }
}

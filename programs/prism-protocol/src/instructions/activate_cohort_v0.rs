use crate::error::ErrorCode;
use crate::state::{CampaignV0, CohortV0};
use crate::{CAMPAIGN_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(
    campaign_fingerprint: [u8; 32],
    cohort_merkle_root: [u8; 32]
)]
pub struct ActivateCohortV0<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [
            CAMPAIGN_V0_SEED_PREFIX,
            admin.key().as_ref(),
            campaign_fingerprint.as_ref()
        ],
        bump = campaign.bump,
        has_one = admin @ ErrorCode::Unauthorized,
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::ConstraintSeedsMismatch,
    )]
    pub campaign: Account<'info, CampaignV0>,

    #[account(
        seeds = [
            COHORT_V0_SEED_PREFIX,
            campaign.key().as_ref(),
            cohort_merkle_root.as_ref(),
        ],
        bump = cohort.bump,
        constraint = cohort.campaign == campaign.key() @ ErrorCode::ConstraintSeedsMismatch,
        constraint = cohort.merkle_root == cohort_merkle_root @ ErrorCode::MerkleRootMismatch,
    )]
    pub cohort: Account<'info, CohortV0>,
}

pub fn handle_activate_cohort_v0(
    ctx: Context<ActivateCohortV0>,
    _campaign_fingerprint: [u8; 32], // consumed in account constraints
    _cohort_merkle_root: [u8; 32],   // consumed in account constraints
) -> Result<()> {
    let cohort = &ctx.accounts.cohort;
    let campaign = &mut ctx.accounts.campaign;

    // Validation 1: All vaults in this cohort must be activated
    require!(
        cohort.activated_vault_count == cohort.expected_vault_count,
        ErrorCode::NotAllVaultsActivated
    );

    // Validation 2: All vaults must be initialized (defensive check)
    require!(
        cohort.initialized_vault_count == cohort.expected_vault_count,
        ErrorCode::VaultNotInitialized
    );

    // Validation 3: Campaign must not be active yet
    require!(!campaign.is_active, ErrorCode::CampaignIsActive);

    // Increment campaign's activated cohort count
    campaign.activated_cohort_count = campaign
        .activated_cohort_count
        .checked_add(1)
        .ok_or(ErrorCode::NumericOverflow)?;

    msg!(
        "Cohort {} activated! ({}/{} vaults activated, {}/{} cohorts activated)",
        cohort.key(),
        cohort.activated_vault_count,
        cohort.expected_vault_count,
        campaign.activated_cohort_count,
        campaign.expected_cohort_count
    );

    Ok(())
}

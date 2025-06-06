use anchor_lang::prelude::*;

use crate::{CampaignStatus, CampaignV0, CohortV0, ErrorCode, COHORT_V0_SEED_PREFIX};

#[derive(Accounts)]
#[instruction(
    merkle_root: [u8; 32],
    amount_per_entitlement: u64,
    expected_vault_count: u8
)]
pub struct InitializeCohortV0<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = admin @ ErrorCode::CampaignAdminMismatch,
        constraint = campaign.status == CampaignStatus::Inactive @ ErrorCode::CampaignIsActive,
    )]
    pub campaign: Account<'info, CampaignV0>,

    #[account(
        init,
        payer = admin,
        space = 8 + CohortV0::INIT_SPACE,
        seeds = [
            COHORT_V0_SEED_PREFIX,
            campaign.key().as_ref(),
            merkle_root.as_ref(),
        ],
        bump
    )]
    pub cohort: Account<'info, CohortV0>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_cohort_v0(
    ctx: Context<InitializeCohortV0>,
    merkle_root: [u8; 32],
    amount_per_entitlement: u64,
    expected_vault_count: u8,
) -> Result<()> {
    require!(expected_vault_count > 0, ErrorCode::NoVaultsExpected);
    require!(amount_per_entitlement > 0, ErrorCode::InvalidEntitlements);

    let cohort = &mut ctx.accounts.cohort;
    cohort.set_inner(CohortV0 {
        campaign: ctx.accounts.campaign.key(),
        merkle_root,
        amount_per_entitlement,
        expected_vault_count,       // Set during cohort initialization
        initialized_vault_count: 0, // Incremented during vault creation
        activated_vault_count: 0,   // Incremented during vault activation
        bump: ctx.bumps.cohort,
    });

    // Increment campaign's initialized cohort count
    let campaign = &mut ctx.accounts.campaign;
    campaign.initialized_cohort_count = campaign
        .initialized_cohort_count
        .checked_add(1)
        .ok_or(ErrorCode::NumericOverflow)?;

    Ok(())
}

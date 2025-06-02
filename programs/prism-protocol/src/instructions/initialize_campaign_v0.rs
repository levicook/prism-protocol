use crate::error::ErrorCode;
use crate::state::{CampaignStatus, CampaignV0};
use crate::CAMPAIGN_V0_SEED_PREFIX;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(campaign_fingerprint: [u8; 32], mint_pubkey: Pubkey, expected_cohort_count: u8)]
pub struct InitializeCampaignV0<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + CampaignV0::INIT_SPACE,
        seeds = [
            CAMPAIGN_V0_SEED_PREFIX,
            admin.key().as_ref(),
            campaign_fingerprint.as_ref()
        ],
        bump
    )]
    pub campaign: Account<'info, CampaignV0>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_campaign_v0(
    ctx: Context<InitializeCampaignV0>,
    campaign_fingerprint: [u8; 32],
    mint: Pubkey,
    expected_cohort_count: u8,
) -> Result<()> {
    require!(expected_cohort_count > 0, ErrorCode::NoCohortsExpected);

    let campaign = &mut ctx.accounts.campaign;
    campaign.set_inner(CampaignV0 {
        admin: ctx.accounts.admin.key(),
        mint,
        fingerprint: campaign_fingerprint,
        campaign_db_ipfs_hash: [0; 32],   // Set during activation
        expected_cohort_count,            // Set during campaign initialization
        initialized_cohort_count: 0,      // Incremented during cohort init
        activated_cohort_count: 0,        // Incremented during cohort activation
        status: CampaignStatus::Inactive, // Starts inactive until activated
        unstoppable: false,               // Starts stoppable, can be made unstoppable later
        go_live_slot: 0,                  // Set during activation
        bump: ctx.bumps.campaign,
    });

    Ok(())
}

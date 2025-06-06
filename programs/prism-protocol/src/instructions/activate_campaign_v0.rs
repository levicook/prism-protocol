use anchor_lang::prelude::*;

use crate::{CampaignStatus, CampaignV0, ErrorCode};

#[derive(Accounts)]
pub struct ActivateCampaignV0<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = admin @ ErrorCode::CampaignAdminMismatch,
        constraint = campaign.status == CampaignStatus::Inactive @ ErrorCode::CampaignAlreadyActivated,
    )]
    pub campaign: Account<'info, CampaignV0>,
}

#[event]
pub struct CampaignActivated {
    pub campaign: Pubkey,
    pub final_db_ipfs_hash: [u8; 32],
    pub go_live_slot: u64,
    pub activated_at_slot: u64,
}

pub fn handle_activate_campaign_v0(
    ctx: Context<ActivateCampaignV0>,
    final_db_ipfs_hash: [u8; 32],
    go_live_slot: u64,
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;
    let current_slot = Clock::get()?.slot;

    // Validation 1: Final IPFS hash must be provided (non-zero)
    require!(final_db_ipfs_hash != [0; 32], ErrorCode::InvalidIpfsHash);

    // Validation 2: Campaign DB IPFS hash must not already be set (prevents re-activation)
    require!(
        campaign.campaign_db_ipfs_hash == [0; 32],
        ErrorCode::CampaignAlreadyActivated
    );

    // Validation 3: Go-live slot must be in the future (or current)
    require!(go_live_slot >= current_slot, ErrorCode::GoLiveSlotInPast);

    // Validation 4: All expected cohorts must be initialized and activated (complete lifecycle)
    require!(
        campaign.activated_cohort_count == campaign.expected_cohort_count
            && campaign.initialized_cohort_count == campaign.expected_cohort_count,
        ErrorCode::NotAllCohortsActivated
    );

    // Validation 5: Must have at least one cohort expected
    require!(
        campaign.expected_cohort_count > 0,
        ErrorCode::NoCohortsExpected
    );

    // Set activation data (immutable once set)
    campaign.campaign_db_ipfs_hash = final_db_ipfs_hash;
    campaign.go_live_slot = go_live_slot;
    campaign.status = CampaignStatus::Active;
    campaign.unstoppable = false; // Always starts stoppable

    // Emit event for backend automation
    emit!(CampaignActivated {
        campaign: campaign.key(),
        final_db_ipfs_hash,
        go_live_slot,
        activated_at_slot: current_slot,
    });

    Ok(())
}

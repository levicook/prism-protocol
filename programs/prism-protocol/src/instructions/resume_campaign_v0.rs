use crate::{
    error::ErrorCode,
    state::{CampaignStatus, CampaignV0},
    CAMPAIGN_V0_SEED_PREFIX,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(campaign_fingerprint: [u8; 32])]
pub struct ResumeCampaignV0<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [
            CAMPAIGN_V0_SEED_PREFIX,
            admin.key().as_ref(),
            campaign_fingerprint.as_ref(),
        ],
        bump = campaign.bump,
        has_one = admin @ ErrorCode::CampaignAdminMismatch,
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::CampaignFingerprintMismatch,
        constraint = campaign.status == CampaignStatus::Paused @ ErrorCode::CampaignNotPaused,
    )]
    pub campaign: Account<'info, CampaignV0>,
}

#[event]
pub struct CampaignResumed {
    pub campaign: Pubkey,
    pub admin: Pubkey,
    pub timestamp: i64,
}

pub fn handle_resume_campaign_v0(
    ctx: Context<ResumeCampaignV0>,
    _campaign_fingerprint: [u8; 32], // Used by Accounts macro for seed derivation
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;

    // Resume the campaign
    campaign.status = CampaignStatus::Active;

    // Emit event
    emit!(CampaignResumed {
        campaign: campaign.key(),
        admin: ctx.accounts.admin.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Campaign {} resumed", campaign.key());

    Ok(())
}

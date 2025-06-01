use crate::{
    error::ErrorCode,
    state::{CampaignStatus, CampaignV0},
    CAMPAIGN_V0_SEED_PREFIX,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(campaign_fingerprint: [u8; 32])]
pub struct PermanentlyHaltCampaignV0<'info> {
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
        constraint = !campaign.unstoppable @ ErrorCode::CampaignIsUnstoppable,
    )]
    pub campaign: Account<'info, CampaignV0>,
}

#[event]
pub struct CampaignPermanentlyHalted {
    pub campaign: Pubkey,
    pub admin: Pubkey,
    pub previous_status: CampaignStatus,
    pub timestamp: i64,
}

pub fn handle_permanently_halt_campaign_v0(
    ctx: Context<PermanentlyHaltCampaignV0>,
    _campaign_fingerprint: [u8; 32], // Used by Accounts macro for seed derivation
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;

    // Validate current status allows halting
    require!(
        campaign.status == CampaignStatus::Active || campaign.status == CampaignStatus::Paused,
        ErrorCode::InvalidStatusTransition
    );

    let previous_status = campaign.status;

    // Permanently halt the campaign
    campaign.status = CampaignStatus::PermanentlyHalted;

    // Emit event
    emit!(CampaignPermanentlyHalted {
        campaign: campaign.key(),
        admin: ctx.accounts.admin.key(),
        previous_status,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Campaign {} permanently halted", campaign.key());

    Ok(())
}

use crate::{
    error::ErrorCode,
    state::{CampaignStatus, CampaignV0},
    CAMPAIGN_V0_SEED_PREFIX,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(campaign_fingerprint: [u8; 32])]
pub struct MakeCampaignUnstoppableV0<'info> {
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
        constraint = campaign.status == CampaignStatus::Active @ ErrorCode::CampaignNotActive,
        constraint = !campaign.unstoppable @ ErrorCode::CampaignIsUnstoppable,
    )]
    pub campaign: Account<'info, CampaignV0>,
}

#[event]
pub struct CampaignMadeUnstoppable {
    pub campaign: Pubkey,
    pub admin: Pubkey,
    pub timestamp: i64,
}

pub fn handle_make_campaign_unstoppable_v0(
    ctx: Context<MakeCampaignUnstoppableV0>,
    _campaign_fingerprint: [u8; 32], // Used by Accounts macro for seed derivation
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;

    // Make campaign unstoppable (irreversible)
    campaign.unstoppable = true;

    // Emit event
    emit!(CampaignMadeUnstoppable {
        campaign: campaign.key(),
        admin: ctx.accounts.admin.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Campaign {} is now unstoppable", campaign.key());

    Ok(())
}

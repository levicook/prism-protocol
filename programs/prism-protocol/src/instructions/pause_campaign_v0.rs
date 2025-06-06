use anchor_lang::prelude::*;

use crate::{CampaignStatus, CampaignV0, ErrorCode};

#[derive(Accounts)]
pub struct PauseCampaignV0<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = admin @ ErrorCode::CampaignAdminMismatch,
        constraint = campaign.status == CampaignStatus::Active @ ErrorCode::CampaignNotActive,
        constraint = !campaign.unstoppable @ ErrorCode::CampaignIsUnstoppable,
    )]
    pub campaign: Account<'info, CampaignV0>,
}

#[event]
pub struct CampaignPaused {
    pub campaign: Pubkey,
    pub admin: Pubkey,
    pub timestamp: i64,
}

pub fn handle_pause_campaign_v0(ctx: Context<PauseCampaignV0>) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;

    // Pause the campaign
    campaign.status = CampaignStatus::Paused;

    // Emit event
    emit!(CampaignPaused {
        campaign: campaign.key(),
        admin: ctx.accounts.admin.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Campaign {} paused", campaign.key());

    Ok(())
}

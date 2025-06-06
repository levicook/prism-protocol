use anchor_lang::prelude::*;

use crate::{CampaignStatus, CampaignV0, ErrorCode};

#[derive(Accounts)]
pub struct ResumeCampaignV0<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = admin @ ErrorCode::CampaignAdminMismatch,
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

pub fn handle_resume_campaign_v0(ctx: Context<ResumeCampaignV0>) -> Result<()> {
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

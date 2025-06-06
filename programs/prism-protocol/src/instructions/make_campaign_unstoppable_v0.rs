use anchor_lang::prelude::*;

use crate::{CampaignStatus, CampaignV0, ErrorCode};

#[derive(Accounts)]
pub struct MakeCampaignUnstoppableV0<'info> {
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
pub struct CampaignMadeUnstoppable {
    pub campaign: Pubkey,
    pub admin: Pubkey,
    pub timestamp: i64,
}

pub fn handle_make_campaign_unstoppable_v0(ctx: Context<MakeCampaignUnstoppableV0>) -> Result<()> {
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

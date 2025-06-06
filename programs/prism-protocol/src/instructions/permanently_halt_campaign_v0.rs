use anchor_lang::prelude::*;

use crate::{CampaignStatus, CampaignV0, ErrorCode};

#[derive(Accounts)]
pub struct PermanentlyHaltCampaignV0<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = admin @ ErrorCode::CampaignAdminMismatch,
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

pub fn handle_permanently_halt_campaign_v0(ctx: Context<PermanentlyHaltCampaignV0>) -> Result<()> {
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

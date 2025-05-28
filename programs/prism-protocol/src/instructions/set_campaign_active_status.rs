use crate::error::ErrorCode;
use crate::state::CampaignV0;
use crate::CAMPAIGN_V0_SEED_PREFIX;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(campaign_fingerprint: [u8; 32], is_active: bool)]
pub struct SetCampaignActiveStatus<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut, // Needs to be mutable to change is_active
        seeds = [
            CAMPAIGN_V0_SEED_PREFIX,
            admin.key().as_ref(),
            campaign_fingerprint.as_ref(),
        ],
        bump = campaign.bump,
        has_one = admin @ ErrorCode::Unauthorized, // Ensures the signer is the campaign authority
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::ConstraintSeedsMismatch,
        constraint = campaign.is_active == false @ ErrorCode::CampaignIsActive
    )]
    pub campaign: Account<'info, CampaignV0>,
}

pub fn handle_set_campaign_active_status(
    ctx: Context<SetCampaignActiveStatus>,
    _campaign_fingerprint: [u8; 32], // Consumed by Accounts macro for seed derivation
    is_active: bool,
) -> Result<()> {
    ctx.accounts.campaign.is_active = is_active;
    Ok(())
}

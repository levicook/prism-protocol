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
            campaign_fingerprint.as_ref(),
        ],
        bump = campaign.bump,
        has_one = admin @ ErrorCode::Unauthorized, // Ensures the signer is the campaign authority
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::ConstraintSeedsMismatch
    )]
    pub campaign: Account<'info, CampaignV0>,
}

pub fn handle_set_campaign_active_status(
    ctx: Context<SetCampaignActiveStatus>,
    _campaign_fingerprint: [u8; 32], // Consumed by Accounts macro for seed derivation
    is_active: bool,
) -> Result<()> {
    ctx.accounts.campaign.is_active = is_active;

    // Optionally, emit an event
    // emit!(CampaignStatusChanged {
    //     campaign_pda: ctx.accounts.campaign.key(),
    //     campaign_fingerprint: ctx.accounts.campaign.fingerprint,
    //     is_active: is_active,
    //     timestamp: Clock::get()?.unix_timestamp,
    // });

    Ok(())
}

// Optional Event Definition:
// #[event]
// pub struct CampaignStatusChanged {
//     pub campaign_pda: Pubkey,
//     pub campaign_fingerprint: [u8; 32],
//     pub is_active: bool,
//     pub timestamp: i64,
// }

use crate::{state::CampaignV0, CAMPAIGN_V0_SEED_PREFIX};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(campaign_fingerprint: [u8; 32], mint_pubkey: Pubkey)]
pub struct InitializeCampaignV0<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + CampaignV0::INIT_SPACE,
        seeds = [
            CAMPAIGN_V0_SEED_PREFIX,
            admin.key().as_ref(),
            campaign_fingerprint.as_ref(),
        ],
        bump
    )]
    pub campaign: Account<'info, CampaignV0>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_campaign_v0(
    ctx: Context<InitializeCampaignV0>,
    campaign_fingerprint: [u8; 32],
    mint: Pubkey,
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;

    campaign.set_inner(CampaignV0 {
        admin: ctx.accounts.admin.key(),
        mint,
        fingerprint: campaign_fingerprint,
        is_active: false,
        bump: ctx.bumps.campaign,
    });

    Ok(())
}

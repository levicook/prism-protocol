use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    CampaignStatus, CampaignV0, CohortV0, ErrorCode, COHORT_V0_SEED_PREFIX, VAULT_SEED_PREFIX,
};

#[derive(Accounts)]
#[instruction(
    cohort_merkle_root: [u8; 32],
    vault_index: u8
)]
pub struct ReclaimTokensV0<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        has_one = admin @ ErrorCode::CampaignAdminMismatch,
    )]
    pub campaign: Account<'info, CampaignV0>,

    #[account(
        seeds = [
            COHORT_V0_SEED_PREFIX,
            campaign.key().as_ref(),
            cohort_merkle_root.as_ref(),
        ],
        bump = cohort.bump,
        constraint = cohort.campaign == campaign.key() @ ErrorCode::CohortCampaignMismatch,
        constraint = cohort.merkle_root == cohort_merkle_root @ ErrorCode::MerkleRootMismatch
    )]
    pub cohort: Account<'info, CohortV0>,

    /// The vault to reclaim tokens from - derived using vault index
    #[account(
        mut,
        seeds = [
            VAULT_SEED_PREFIX,
            cohort.key().as_ref(),
            &vault_index.to_le_bytes()
        ],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        // Constraint: must be owned by the admin signer to ensure funds go to the right place.
        constraint = destination_token_account.owner == admin.key() @ ErrorCode::TokenAccountOwnerMismatch
    )]
    pub destination_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_reclaim_tokens_v0(
    ctx: Context<ReclaimTokensV0>,
    cohort_merkle_root: [u8; 32],
    vault_index: u8,
) -> Result<()> {
    let campaign = &ctx.accounts.campaign;
    let cohort = &ctx.accounts.cohort;

    // Validation 1: Campaign must be permanently halted to reclaim tokens
    require!(
        campaign.status == CampaignStatus::PermanentlyHalted,
        ErrorCode::CampaignNotPermanentlyHalted
    );

    // Validation 2: Validate vault index is within expected range
    require!(
        vault_index < cohort.expected_vault_count,
        ErrorCode::VaultIndexOutOfBounds
    );

    let amount_to_reclaim = ctx.accounts.vault.amount;

    if amount_to_reclaim == 0 {
        // Or return Ok(()) if withdrawing 0 is acceptable.
        // For now, let's assume withdrawing 0 is not an error but nothing happens.
        msg!("Vault is empty, no tokens to reclaim.");
        return Ok(());
    }

    let transfer_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.destination_token_account.to_account_info(),
        authority: ctx.accounts.cohort.to_account_info(),
    };

    // Use cohort PDA signer seeds (cohort owns the token vaults)
    let campaign_key = ctx.accounts.campaign.key();
    let cohort_seeds = &[
        COHORT_V0_SEED_PREFIX,
        campaign_key.as_ref(),
        cohort_merkle_root.as_ref(),
        &[ctx.accounts.cohort.bump],
    ];
    let signer_seeds = &[&cohort_seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        ),
        amount_to_reclaim,
    )?;

    Ok(())
}

use crate::constants::VAULT_SEED_PREFIX;
use crate::error::ErrorCode;
use crate::state::{CampaignStatus, CampaignV0, CohortV0};
use crate::{CAMPAIGN_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

#[derive(Accounts)]
#[instruction(
    campaign_fingerprint: [u8; 32],
    cohort_merkle_root: [u8; 32],
    vault_index: u8
)]
pub struct ReclaimTokensV0<'info> {
    #[account(mut)] // admin pays for tx, not mutated itself
    pub admin: Signer<'info>,

    #[account(
        seeds = [
            CAMPAIGN_V0_SEED_PREFIX,
            admin.key().as_ref(),
            campaign_fingerprint.as_ref(),
        ],
        bump = campaign.bump,
        has_one = admin @ ErrorCode::CampaignAdminMismatch,
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::CampaignFingerprintMismatch,
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
    _campaign_fingerprint: [u8; 32],   // Consumed by Accounts macro
    _cohort_merkle_root_arg: [u8; 32], // Consumed by Accounts macro
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
        _cohort_merkle_root_arg.as_ref(),
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

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    CampaignStatus, CampaignV0, CohortV0, ErrorCode, COHORT_V0_SEED_PREFIX, VAULT_SEED_PREFIX,
};

#[derive(Accounts)]
#[instruction(
    cohort_merkle_root: [u8; 32],
    vault_index: u8
)]
pub struct InitializeVaultV0<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        has_one = admin @ ErrorCode::CampaignAdminMismatch,
        constraint = campaign.status == CampaignStatus::Inactive @ ErrorCode::CampaignIsActive,
    )]
    pub campaign: Account<'info, CampaignV0>,

    #[account(
        mut,
        seeds = [
            COHORT_V0_SEED_PREFIX,
            campaign.key().as_ref(),
            cohort_merkle_root.as_ref(),
        ],
        bump = cohort.bump,
        constraint = cohort.campaign == campaign.key() @ ErrorCode::CohortCampaignMismatch,
        constraint = cohort.merkle_root == cohort_merkle_root @ ErrorCode::MerkleRootMismatch,
    )]
    pub cohort: Account<'info, CohortV0>,

    /// The mint for the token accounts being created
    #[account(
        constraint = mint.key() == campaign.mint @ ErrorCode::MintMismatch
    )]
    pub mint: Account<'info, Mint>,

    /// The vault (token account) to be created
    #[account(
        init,
        payer = admin,
        token::mint = mint,
        token::authority = cohort, // Cohort PDA signs transfers from this vault
        seeds = [
            VAULT_SEED_PREFIX,
            cohort.key().as_ref(),
            &vault_index.to_le_bytes()
        ],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_vault_v0(
    ctx: Context<InitializeVaultV0>,
    _cohort_merkle_root: [u8; 32], // consumed in account constraints
    vault_index: u8,
) -> Result<()> {
    let cohort = &mut ctx.accounts.cohort;

    // Validation 1: Vault index must be within expected range
    require!(
        vault_index < cohort.expected_vault_count,
        ErrorCode::VaultIndexOutOfBounds
    );

    // Validation 2: Cannot initialize more vaults than expected
    require!(
        cohort.initialized_vault_count < cohort.expected_vault_count,
        ErrorCode::TooManyVaults
    );

    // Increment initialized vault count
    cohort.initialized_vault_count = cohort
        .initialized_vault_count
        .checked_add(1)
        .ok_or(ErrorCode::NumericOverflow)?;

    msg!(
        "Initialized vault {} for cohort {} at address {}",
        vault_index,
        ctx.accounts.cohort.key(),
        ctx.accounts.vault.key()
    );

    Ok(())
}

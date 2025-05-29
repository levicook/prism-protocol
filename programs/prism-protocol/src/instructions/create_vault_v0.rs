use crate::constants::VAULT_SEED_PREFIX;
use crate::error::ErrorCode;
use crate::state::{CampaignV0, CohortV0};
use crate::{CAMPAIGN_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
#[instruction(
    campaign_fingerprint: [u8; 32],
    cohort_merkle_root: [u8; 32],
    vault_index: u8
)]
pub struct CreateVaultV0<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [
            CAMPAIGN_V0_SEED_PREFIX,
            admin.key().as_ref(),
            campaign_fingerprint.as_ref()
        ],
        bump = campaign.bump,
        has_one = admin @ ErrorCode::Unauthorized,
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::ConstraintSeedsMismatch,
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
        constraint = cohort.campaign == campaign.key() @ ErrorCode::InvalidMerkleProof,
        constraint = cohort.merkle_root == cohort_merkle_root @ ErrorCode::MerkleRootMismatch,
    )]
    pub cohort: Account<'info, CohortV0>,

    /// The mint for the token accounts being created
    #[account(
        constraint = mint.key() == campaign.mint @ ErrorCode::InvalidMerkleProof
    )]
    // completely wrong error code ^^^
    pub mint: Account<'info, Mint>,

    /// The vault (token account) to be created
    /// This is a PDA derived from the cohort and vault index
    #[account(
        init,
        payer = admin,
        token::mint = mint,
        token::authority = cohort, // Cohort PDA signs transfers from this vault
        seeds = [
            VAULT_SEED_PREFIX,
            cohort.key().as_ref(),
            &[vault_index]
        ],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_create_vault_v0(
    ctx: Context<CreateVaultV0>,
    _campaign_fingerprint: [u8; 32], // consumed in account constraints
    _cohort_merkle_root: [u8; 32],   // consumed in account constraints
    vault_index: u8,
) -> Result<()> {
    let cohort = &mut ctx.accounts.cohort;

    // Ensure vault_index is within bounds
    require!(
        (vault_index as usize) < cohort.vaults.len(),
        ErrorCode::InvalidVaultIndex
    );

    // Ensure this vault hasn't been created yet
    require!(
        cohort.vaults[vault_index as usize] == Pubkey::default(),
        ErrorCode::VaultAlreadyExists
    );

    // Assign the token account to the specific index
    cohort.vaults[vault_index as usize] = ctx.accounts.vault.key();

    msg!(
        "Created vault {} for cohort {} at address {}",
        vault_index,
        ctx.accounts.cohort.key(),
        ctx.accounts.vault.key()
    );

    Ok(())
}

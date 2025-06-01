use crate::constants::VAULT_SEED_PREFIX;
use crate::error::ErrorCode;
use crate::state::{CampaignV0, CohortV0};
use crate::{CAMPAIGN_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX};
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

#[derive(Accounts)]
#[instruction(
    campaign_fingerprint: [u8; 32],
    cohort_merkle_root: [u8; 32],
    vault_index: u8
)]
pub struct ActivateVaultV0<'info> {
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
        constraint = cohort.campaign == campaign.key() @ ErrorCode::ConstraintSeedsMismatch,
        constraint = cohort.merkle_root == cohort_merkle_root @ ErrorCode::MerkleRootMismatch,
    )]
    pub cohort: Account<'info, CohortV0>,

    /// The vault (token account) to activate
    #[account(
        seeds = [
            VAULT_SEED_PREFIX,
            cohort.key().as_ref(),
            &vault_index.to_le_bytes()
        ],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,
}

pub fn handle_activate_vault_v0(
    ctx: Context<ActivateVaultV0>,
    _campaign_fingerprint: [u8; 32], // consumed in account constraints
    _cohort_merkle_root: [u8; 32],   // consumed in account constraints
    vault_index: u8,
    expected_balance: u64,
) -> Result<()> {
    let cohort = &mut ctx.accounts.cohort;
    let vault = &ctx.accounts.vault;

    // Validation 1: Vault index must be within expected range
    require!(
        vault_index < cohort.expected_vault_count,
        ErrorCode::InvalidVaultIndex
    );

    // Validation 2: Cannot activate more vaults than initialized
    require!(
        cohort.activated_vault_count < cohort.initialized_vault_count,
        ErrorCode::VaultNotInitialized
    );

    // Validation 3: Vault must be funded with exactly the expected amount
    require!(
        vault.amount == expected_balance,
        ErrorCode::IncorrectVaultFunding
    );

    // Increment activated vault count
    cohort.activated_vault_count = cohort
        .activated_vault_count
        .checked_add(1)
        .ok_or(ErrorCode::NumericOverflow)?;

    msg!(
        "Activated vault {} for cohort {} with balance {}",
        vault_index,
        ctx.accounts.cohort.key(),
        vault.amount
    );

    Ok(())
}

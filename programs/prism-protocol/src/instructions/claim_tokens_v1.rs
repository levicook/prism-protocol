use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::VAULT_SEED_PREFIX;
use crate::error::ErrorCode;
use crate::instructions::claim_tokens_common::handle_claim_tokens_common;
use crate::proofs::ClaimProofType;
use crate::state::{CampaignStatus, CampaignV0, ClaimReceiptV0, CohortV0};
use crate::{CAMPAIGN_V0_SEED_PREFIX, CLAIM_RECEIPT_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX};

#[derive(Accounts)]
#[instruction(
    campaign_fingerprint: [u8; 32], // Used to find Campaign PDA
    merkle_root: [u8; 32], // Used to find Cohort PDA (this is the cohort.merkle_root)
    merkle_proof: Vec<Vec<[u8; 32]>>, // 256-ary tree proof structure
    assigned_vault_index: u8,
    entitlements: u64
)]
pub struct ClaimTokensV1<'info> {
    /// CHECK: This account is validated through the campaign PDA seeds constraint.
    /// The admin key is used as a seed for deriving the campaign PDA, ensuring
    /// that only the correct admin can be used for the specific campaign.
    #[account()]
    pub admin: UncheckedAccount<'info>,

    /// The person claiming the tokens. This account will sign the transaction.
    #[account(mut)]
    pub claimant: Signer<'info>,

    #[account(
        seeds = [
            CAMPAIGN_V0_SEED_PREFIX,
            admin.key().as_ref(),
            campaign_fingerprint.as_ref(),
        ],
        bump = campaign.bump,
        constraint = campaign.status == CampaignStatus::Active @ ErrorCode::CampaignNotActive,
        constraint = campaign.mint == mint.key() @ ErrorCode::MintMismatch,
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::CampaignFingerprintMismatch,
    )]
    pub campaign: Box<Account<'info, CampaignV0>>,

    #[account(
        seeds = [
            COHORT_V0_SEED_PREFIX,
            campaign.key().as_ref(),
            merkle_root.as_ref()
        ],
        bump = cohort.bump,
        constraint = cohort.campaign == campaign.key() @ ErrorCode::CohortCampaignMismatch,
        constraint = cohort.merkle_root == merkle_root @ ErrorCode::MerkleRootMismatch,
    )]
    pub cohort: Box<Account<'info, CohortV0>>,

    /// The specific vault from which tokens will be transferred.
    /// The vault pubkey is derived using the vault index.
    #[account(
        mut,
        constraint = vault.mint == mint.key() @ ErrorCode::MintMismatch,
        seeds = [
            VAULT_SEED_PREFIX,
            cohort.key().as_ref(),
            &assigned_vault_index.to_le_bytes()
        ],
        bump
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    /// The mint of the token being distributed.
    #[account(
        constraint = mint.key() == campaign.mint @ ErrorCode::MintMismatch
    )]
    pub mint: Box<Account<'info, Mint>>,

    /// The claimant's token account where the rewards will be sent.
    #[account(
        init_if_needed,
        payer = claimant,
        associated_token::mint = mint,
        associated_token::authority = claimant,
    )]
    pub claimant_token_account: Box<Account<'info, TokenAccount>>,

    /// PDA to store the claim receipt, preventing replays.
    #[account(
        init,
        payer = claimant,
        space = 8 + ClaimReceiptV0::INIT_SPACE,
        seeds = [
            CLAIM_RECEIPT_V0_SEED_PREFIX,
            cohort.key().as_ref(),
            claimant.key().as_ref()
        ],
        bump
    )]
    pub claim_receipt: Box<Account<'info, ClaimReceiptV0>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_claim_tokens_v1(
    ctx: Context<ClaimTokensV1>,
    _campaign_fingerprint: [u8; 32], // Consumed by Accounts macro for seed derivation
    cohort_merkle_root: [u8; 32], // Consumed by Accounts macro for seed derivation, also checked in constraint
    merkle_proof: Vec<Vec<[u8; 32]>>, // 256-ary tree proof
    assigned_vault_index: u8,
    entitlements: u64,
) -> Result<()> {
    // Create proof type for 256-ary tree
    let proof = ClaimProofType::from_wide(merkle_proof);

    // Delegate to common handler
    handle_claim_tokens_common(
        &ctx.accounts.claimant,
        &ctx.accounts.campaign,
        &ctx.accounts.cohort,
        &mut ctx.accounts.vault,
        &mut ctx.accounts.claimant_token_account,
        &mut ctx.accounts.claim_receipt,
        &ctx.accounts.token_program,
        cohort_merkle_root,
        proof,
        assigned_vault_index,
        entitlements,
        ctx.bumps.claim_receipt,
    )
} 
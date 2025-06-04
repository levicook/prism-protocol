use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::claim_leaf::ClaimLeaf;
use crate::constants::VAULT_SEED_PREFIX;
use crate::error::ErrorCode;
use crate::proofs::ProofV0;
use crate::state::{CampaignStatus, CampaignV0, ClaimReceiptV0, CohortV0};
use crate::{CAMPAIGN_V0_SEED_PREFIX, CLAIM_RECEIPT_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX};

#[derive(Accounts)]
#[instruction(
    campaign_fingerprint: [u8; 32], // Used to find Campaign PDA
    merkle_root: [u8; 32], // Used to find Cohort PDA (this is the cohort.merkle_root)
    merkle_proof: Vec<[u8; 32]>,
    assigned_vault_index: u8,
    entitlements: u64
)]
pub struct ClaimTokensV0<'info> {
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

    /// The mint of the token being distributed. Renamed from reward_token_mint.
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

// Renamed from handler_with_proper_proof_verification
pub fn handle_claim_tokens_v0(
    ctx: Context<ClaimTokensV0>,
    _campaign_fingerprint: [u8; 32], // Consumed by Accounts macro for seed derivation
    cohort_merkle_root: [u8; 32], // Consumed by Accounts macro for seed derivation, also checked in constraint
    merkle_proof: Vec<[u8; 32]>,
    assigned_vault_index: u8,
    entitlements: u64,
) -> Result<()> {
    // 0. Basic argument validation
    require!(entitlements > 0, ErrorCode::InvalidEntitlements);

    // 1. Check campaign go-live slot
    let campaign = &ctx.accounts.campaign;
    let current_slot = Clock::get()?.slot;
    require!(
        current_slot >= campaign.go_live_slot,
        ErrorCode::GoLiveDateNotReached
    );

    let cohort = &ctx.accounts.cohort;

    // 2. Validate vault index is within bounds (using expected vault count)
    require!(
        assigned_vault_index < cohort.expected_vault_count,
        ErrorCode::AssignedVaultIndexOutOfBounds
    );

    // 3. Construct the leaf node from the transaction data
    let leaf = ClaimLeaf {
        claimant: ctx.accounts.claimant.key(),
        assigned_vault_index,
        entitlements,
    };

    // 4. Verify the Merkle proof using our hashing scheme (SHA256)
    let proof = ProofV0::new(merkle_proof);
    if !proof.verify(&cohort.merkle_root, &leaf) {
        return err!(ErrorCode::InvalidMerkleProof);
    }
    msg!("Merkle proof verified successfully.");

    // 5. Check if already claimed (ClaimReceipt PDA is initialized, so this prevents re-init)
    // The init constraint on ClaimReceipt already handles this.

    // 6. Calculate total tokens to be claimed
    let total_amount = cohort
        .amount_per_entitlement
        .checked_mul(entitlements)
        .ok_or(ErrorCode::NumericOverflow)?;

    // 7. Perform the token transfer
    let transfer_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.claimant_token_account.to_account_info(),
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
        total_amount,
    )?;

    // 8. Update state
    let claim_receipt = &mut ctx.accounts.claim_receipt;
    claim_receipt.set_inner(ClaimReceiptV0 {
        cohort: cohort.key(),
        claimant: ctx.accounts.claimant.key(),
        assigned_vault: ctx.accounts.vault.key(),
        claimed_at_timestamp: Clock::get()?.unix_timestamp,
        bump: ctx.bumps.claim_receipt,
    });

    Ok(())
}

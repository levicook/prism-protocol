use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::error::ErrorCode;
use crate::merkle_leaf::{hash_claim_leaf, ClaimLeaf};
use crate::state::{CampaignV0, ClaimReceiptV0, CohortV0};
use crate::CAMPAIGN_V0_SEED_PREFIX;
use anchor_lang::solana_program::hash::Hasher as SolanaHasher; // Alias to avoid conflict if Hasher trait is in scope elsewhere

#[derive(Accounts)]
#[instruction(
    campaign_fingerprint: [u8; 32], // New: Used to find Campaign PDA
    cohort_merkle_root: [u8; 32],         // New: Used to find Cohort PDA (this is the cohort.merkle_root)
    merkle_proof: Vec<[u8; 32]>,
    assigned_vault_from_leaf: Pubkey,
    entitlements_from_leaf: u64
)]
pub struct ClaimTokensV0<'info> {
    /// The person claiming the reward. This account will sign the transaction.
    #[account(mut)]
    pub claimant: Signer<'info>,

    #[account(
        seeds = [CAMPAIGN_V0_SEED_PREFIX, campaign_fingerprint.as_ref()],
        bump = campaign.bump,
        constraint = campaign.is_active @ ErrorCode::CampaignNotActive,
        constraint = campaign.mint == mint.key() @ ErrorCode::InvalidMerkleProof,
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::ConstraintSeedsMismatch,
    )]
    pub campaign: Box<Account<'info, CampaignV0>>,

    #[account(
        seeds = [b"cohort".as_ref(), campaign.key().as_ref(), cohort_merkle_root.as_ref()],
        bump = cohort.bump,
        constraint = cohort.campaign == campaign.key() @ ErrorCode::InvalidMerkleProof,
        constraint = cohort.merkle_root == cohort_merkle_root @ ErrorCode::MerkleRootMismatch, // Ensure cohort found by seed matches arg
    )]
    pub cohort: Box<Account<'info, CohortV0>>,

    /// The specific vault token account from which tokens will be transferred.
    /// Renamed from reward_token_vault to token_vault.
    #[account(
        mut,
        constraint = token_vault.mint == mint.key() @ ErrorCode::InvalidMerkleProof,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,

    /// The mint of the token being distributed. Renamed from reward_token_mint.
    #[account(
        constraint = mint.key() == campaign.mint @ ErrorCode::InvalidMerkleProof
    )]
    pub mint: Box<Account<'info, Mint>>,

    /// The claimant's token account where the rewards will be sent.
    #[account(
        init_if_needed,
        payer = claimant,
        associated_token::mint = mint, // Use the renamed mint field
        associated_token::authority = claimant,
    )]
    pub claimant_token_account: Box<Account<'info, TokenAccount>>,

    /// PDA to store the claim receipt, preventing replays.
    #[account(
        init,
        payer = claimant,
        space = 8 + ClaimReceiptV0::INIT_SPACE,
        seeds = [b"claim_receipt".as_ref(), cohort.key().as_ref(), claimant.key().as_ref()],
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
    campaign_fingerprint: [u8; 32], // Consumed by Accounts macro for seed derivation
    _cohort_merkle_root_arg: [u8; 32], // Consumed by Accounts macro for seed derivation, also checked in constraint
    merkle_proof: Vec<[u8; 32]>,
    assigned_vault_from_leaf: Pubkey,
    entitlements_from_leaf: u64,
) -> Result<()> {
    // 0. Basic argument validation
    require!(entitlements_from_leaf > 0, ErrorCode::InvalidMerkleProof);
    require!(
        ctx.accounts.token_vault.key() == assigned_vault_from_leaf,
        ErrorCode::InvalidAssignedVault
    );

    // 1. Verify the assigned_vault_from_leaf is part of the cohort's official vaults
    let cohort = &ctx.accounts.cohort;
    require!(
        cohort.vaults.contains(&assigned_vault_from_leaf),
        ErrorCode::InvalidAssignedVault
    );

    // 2. Construct the leaf node from the transaction data
    let leaf = ClaimLeaf {
        claimant: ctx.accounts.claimant.key(),
        assigned_vault: assigned_vault_from_leaf,
        entitlements: entitlements_from_leaf,
    };
    let leaf_hash = hash_claim_leaf(&leaf);

    // 3. Verify the Merkle proof using SPL standard (SHA256)
    if !verify_spl_merkle_proof(&merkle_proof, &cohort.merkle_root, &leaf_hash) {
        return err!(ErrorCode::InvalidMerkleProof);
    }
    msg!("Merkle proof verified successfully.");

    // 4. Check if already claimed (ClaimReceipt PDA is initialized, so this prevents re-init)
    // The init constraint on ClaimReceipt already handles this.

    // 5. Calculate total reward
    let total_reward_amount = cohort
        .reward_per_entitlement
        .checked_mul(entitlements_from_leaf)
        .ok_or(ErrorCode::NumericOverflow)?;

    // 6. Perform the token transfer
    let transfer_accounts = Transfer {
        from: ctx.accounts.token_vault.to_account_info(),
        to: ctx.accounts.claimant_token_account.to_account_info(),
        authority: ctx.accounts.campaign.to_account_info(),
    };

    // Use campaign_fingerprint for campaign PDA signer seeds
    let campaign_seeds = &[
        CAMPAIGN_V0_SEED_PREFIX,
        campaign_fingerprint.as_ref(), // Use the arg passed to instruction
        &[ctx.accounts.campaign.bump],
    ];
    let signer_seeds = &[&campaign_seeds[..]];
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        ),
        total_reward_amount,
    )?;

    // 7. Update state
    // cohort.claimed_entitlements_count was removed.

    let claim_receipt = &mut ctx.accounts.claim_receipt;
    claim_receipt.cohort_account = cohort.key();
    claim_receipt.claimant = ctx.accounts.claimant.key();
    claim_receipt.assigned_vault = assigned_vault_from_leaf;
    claim_receipt.claimed_at_timestamp = Clock::get()?.unix_timestamp;
    // claim_receipt.bump is handled by Anchor for init accounts

    emit!(ClaimEvent {
        campaign: ctx.accounts.campaign.key(),
        cohort: cohort.key(),
        claimant: ctx.accounts.claimant.key(),
        assigned_vault: assigned_vault_from_leaf,
        mint: ctx.accounts.mint.key(),
        claimed_entitlements: entitlements_from_leaf,
        total_reward_amount,
        timestamp: claim_receipt.claimed_at_timestamp,
    });

    Ok(())
}

#[event]
pub struct ClaimEvent {
    pub campaign: Pubkey,
    pub cohort: Pubkey,
    pub claimant: Pubkey,
    pub assigned_vault: Pubkey,
    pub mint: Pubkey, // Renamed from reward_token_mint
    pub claimed_entitlements: u64,
    pub total_reward_amount: u64,
    pub timestamp: i64,
}

/// Verifies a Merkle proof using SPL standard (SHA256 hashing).
/// - Leaf nodes are hashed as: SHA256(0x00 || borsh_serialized_leaf_data)
/// - Internal nodes are hashed as: SHA256(0x01 || H(LeftChild) || H(RightChild))
///   Child hashes are typically ordered lexicographically before concatenation.
fn verify_spl_merkle_proof(proof: &[[u8; 32]], root: &[u8; 32], leaf: &[u8; 32]) -> bool {
    let mut computed_hash = *leaf;
    for p_elem in proof.iter() {
        let mut hasher = SolanaHasher::default(); // Uses SHA256 by default
        hasher.hash(&[0x01]); // Node prefix
                              // Correctly order H(L) and H(R) before hashing for the parent node.
                              // This order must match the tree generation logic (e.g., rs-merkle sorts hashes lexicographically).
        if computed_hash.as_ref() <= p_elem.as_ref() {
            hasher.hash(&computed_hash);
            hasher.hash(p_elem);
        } else {
            hasher.hash(p_elem);
            hasher.hash(&computed_hash);
        }
        computed_hash = hasher.result().to_bytes();
    }
    computed_hash.as_ref() == root
}

// Original handler with placeholder/incorrect Keccak logic has been removed.

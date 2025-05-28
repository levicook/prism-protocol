use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::error::ErrorCode;
use crate::merkle_leaf::{hash_claim_leaf, ClaimLeaf};
use crate::state::{CampaignV0, ClaimReceiptV0, CohortV0};
use crate::{CAMPAIGN_V0_SEED_PREFIX, CLAIM_RECEIPT_V0_SEED_PREFIX, COHORT_V0_SEED_PREFIX};
use anchor_lang::solana_program::hash::Hasher as SolanaHasher; // Alias to avoid conflict if Hasher trait is in scope elsewhere

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
        constraint = campaign.is_active @ ErrorCode::CampaignNotActive,
        constraint = campaign.mint == mint.key() @ ErrorCode::InvalidMerkleProof,
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::ConstraintSeedsMismatch,
    )]
    pub campaign: Box<Account<'info, CampaignV0>>,

    #[account(
        seeds = [
            COHORT_V0_SEED_PREFIX,
            campaign.key().as_ref(),
            merkle_root.as_ref()
        ],
        bump = cohort.bump,
        constraint = cohort.campaign == campaign.key() @ ErrorCode::InvalidMerkleProof,
        constraint = cohort.merkle_root == merkle_root @ ErrorCode::MerkleRootMismatch, // Ensure cohort found by seed matches arg
    )]
    pub cohort: Box<Account<'info, CohortV0>>,

    /// The specific vault token account from which tokens will be transferred.
    /// The vault pubkey is derived from the cohort.vaults[assigned_vault_index].
    #[account(
        mut,
        constraint = token_vault.mint == mint.key() @ ErrorCode::InvalidMerkleProof,
        constraint = token_vault.key() == cohort.vaults[assigned_vault_index as usize] @ ErrorCode::InvalidAssignedVault,
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
    _cohort_merkle_root: [u8; 32], // Consumed by Accounts macro for seed derivation, also checked in constraint
    merkle_proof: Vec<[u8; 32]>,
    assigned_vault_index: u8,
    entitlements: u64,
) -> Result<()> {
    // 0. Basic argument validation
    require!(entitlements > 0, ErrorCode::InvalidMerkleProof);

    let cohort = &ctx.accounts.cohort;

    // 1. Validate vault index is within bounds
    require!(
        (assigned_vault_index as usize) < cohort.vaults.len(),
        ErrorCode::InvalidAssignedVault
    );

    // 2. Get the vault pubkey from the index (already validated by constraint)
    let assigned_vault_pubkey = cohort.vaults[assigned_vault_index as usize];

    // 3. Construct the leaf node from the transaction data
    let leaf = ClaimLeaf {
        claimant: ctx.accounts.claimant.key(),
        assigned_vault_index,
        entitlements,
    };
    let leaf_hash = hash_claim_leaf(&leaf);

    // 4. Verify the Merkle proof using our hashing scheme (SHA256)
    if !verify_merkle_proof(&merkle_proof, &cohort.merkle_root, &leaf_hash) {
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
        from: ctx.accounts.token_vault.to_account_info(),
        to: ctx.accounts.claimant_token_account.to_account_info(),
        authority: ctx.accounts.cohort.to_account_info(),
    };

    // Use cohort PDA signer seeds (cohort owns the token vaults)
    let campaign_key = ctx.accounts.campaign.key();
    let cohort_seeds = &[
        COHORT_V0_SEED_PREFIX,
        campaign_key.as_ref(),
        _cohort_merkle_root.as_ref(),
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
        assigned_vault: assigned_vault_pubkey,
        claimed_at_timestamp: Clock::get()?.unix_timestamp,
        bump: ctx.bumps.claim_receipt,
    });

    Ok(())
}

/// Verifies a Merkle proof using our hashing scheme (SHA256 hashing).
///
/// ## Security: Domain Separation
/// This function enforces the same prefix-based domain separation as our off-chain
/// merkle tree generation to prevent second preimage attacks:
/// - Leaf nodes are hashed as: SHA256(0x00 || borsh_serialized_leaf_data)
/// - Internal nodes are hashed as: SHA256(0x01 || H(LeftChild) || H(RightChild))
/// - Child hashes are ordered lexicographically before concatenation.
///
/// The prefix bytes ensure that leaf hashes can never equal internal node hashes,
/// preventing attackers from forging proofs by substituting node types.
fn verify_merkle_proof(proof: &[[u8; 32]], root: &[u8; 32], leaf: &[u8; 32]) -> bool {
    let mut computed_hash = *leaf;
    for p_elem in proof.iter() {
        let mut hasher = SolanaHasher::default(); // Uses SHA256 by default
        hasher.hash(&[0x01]); // Internal node prefix - provides domain separation from leaf nodes (0x00)
                              // Correctly order H(L) and H(R) before hashing for the parent node.
                              // This order must match the tree generation logic (lexicographic ordering).
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

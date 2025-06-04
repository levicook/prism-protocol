use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::claim_leaf::ClaimLeaf;
use crate::error::ErrorCode;
use crate::proofs::ClaimProofType;
use crate::state::{CampaignV0, ClaimReceiptV0, CohortV0};
use crate::COHORT_V0_SEED_PREFIX;

/// Common implementation for both claim_tokens_v0 and claim_tokens_v1.
/// 
/// This function contains all the shared logic between the two claim handlers,
/// with the only difference being the proof type verification.
pub(crate) fn handle_claim_tokens_common<'info>(
    claimant: &Signer<'info>,
    campaign: &Account<'info, CampaignV0>,
    cohort: &Account<'info, CohortV0>,
    vault: &mut Account<'info, TokenAccount>,
    claimant_token_account: &mut Account<'info, TokenAccount>,
    claim_receipt: &mut Account<'info, ClaimReceiptV0>,
    token_program: &Program<'info, Token>,
    cohort_merkle_root: [u8; 32],
    proof: ClaimProofType,
    assigned_vault_index: u8,
    entitlements: u64,
    claim_receipt_bump: u8,
) -> Result<()> {
    // 0. Basic argument validation
    require!(entitlements > 0, ErrorCode::InvalidEntitlements);

    // 1. Check campaign go-live slot
    let current_slot = Clock::get()?.slot;
    require!(
        current_slot >= campaign.go_live_slot,
        ErrorCode::GoLiveDateNotReached
    );

    // 2. Validate vault index is within bounds (using expected vault count)
    require!(
        assigned_vault_index < cohort.expected_vault_count,
        ErrorCode::AssignedVaultIndexOutOfBounds
    );

    // 3. Construct the leaf node from the transaction data
    let leaf = ClaimLeaf {
        claimant: claimant.key(),
        assigned_vault_index,
        entitlements,
    };

    // 4. Verify the Merkle proof using our hashing scheme (SHA256)
    if !proof.verify(&cohort.merkle_root, &leaf) {
        return err!(ErrorCode::InvalidMerkleProof);
    }
    msg!("{} verified successfully.", proof.description());

    // 5. Check if already claimed (ClaimReceipt PDA is initialized, so this prevents re-init)
    // The init constraint on ClaimReceipt already handles this.

    // 6. Calculate total tokens to be claimed
    let total_amount = cohort
        .amount_per_entitlement
        .checked_mul(entitlements)
        .ok_or(ErrorCode::NumericOverflow)?;

    // 7. Perform the token transfer
    let transfer_accounts = Transfer {
        from: vault.to_account_info(),
        to: claimant_token_account.to_account_info(),
        authority: cohort.to_account_info(),
    };

    // Use cohort PDA signer seeds (cohort owns the token vaults)
    let campaign_key = campaign.key();
    let cohort_seeds = &[
        COHORT_V0_SEED_PREFIX,
        campaign_key.as_ref(),
        cohort_merkle_root.as_ref(),
        &[cohort.bump],
    ];

    let signer_seeds = &[&cohort_seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        ),
        total_amount,
    )?;

    // 8. Update state
    claim_receipt.set_inner(ClaimReceiptV0 {
        cohort: cohort.key(),
        claimant: claimant.key(),
        assigned_vault: vault.key(),
        claimed_at_timestamp: Clock::get()?.unix_timestamp,
        bump: claim_receipt_bump,
    });

    Ok(())
} 
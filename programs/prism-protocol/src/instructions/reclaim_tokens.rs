use crate::error::ErrorCode;
use crate::state::{CampaignV0, CohortV0};
use crate::CAMPAIGN_V0_SEED_PREFIX;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

#[derive(Accounts)]
#[instruction(
    campaign_fingerprint: [u8; 32],
    cohort_merkle_root: [u8; 32]
)]
pub struct ReclaimTokens<'info> {
    #[account(mut)] // admin pays for tx, not mutated itself
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"campaign".as_ref(), campaign_fingerprint.as_ref()],
        bump = campaign.bump,
        has_one = admin @ ErrorCode::Unauthorized,
        constraint = campaign.fingerprint == campaign_fingerprint @ ErrorCode::ConstraintSeedsMismatch,
        constraint = !campaign.is_active @ ErrorCode::CampaignIsStillActive // Crucial: Campaign must be inactive
    )]
    pub campaign: Account<'info, CampaignV0>,

    #[account(
        seeds = [b"cohort".as_ref(), campaign.key().as_ref(), cohort_merkle_root.as_ref()],
        bump = cohort.bump,
        constraint = cohort.campaign == campaign.key() @ ErrorCode::InvalidMerkleProof, // Basic integrity check
        constraint = cohort.merkle_root == cohort_merkle_root @ ErrorCode::MerkleRootMismatch
    )]
    pub cohort: Account<'info, CohortV0>,

    #[account(
        mut,
        constraint = cohort.vaults.contains(&token_vault_to_reclaim_from.key()) @ ErrorCode::InvalidAssignedVault,
        // The vault's admin should be the Campaign PDA.
        // This is implicitly handled because the Campaign PDA will sign the transfer.
    )]
    pub token_vault_to_reclaim_from: Account<'info, TokenAccount>,

    #[account(
        mut,
        // Constraint: must be owned by the admin signer to ensure funds go to the right place.
        constraint = destination_token_account.owner == admin.key() @ ErrorCode::InvalidAuthority
    )]
    pub destination_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_reclaim_tokens(
    ctx: Context<ReclaimTokens>,
    campaign_fingerprint: [u8; 32],
    _cohort_merkle_root_arg: [u8; 32], // Consumed by Accounts macro
) -> Result<()> {
    let amount_to_reclaim = ctx.accounts.token_vault_to_reclaim_from.amount;

    if amount_to_reclaim == 0 {
        // Or return Ok(()) if withdrawing 0 is acceptable.
        // For now, let's assume withdrawing 0 is not an error but nothing happens.
        msg!("Vault is empty, no tokens to reclaim.");
        return Ok(());
    }

    let transfer_accounts = Transfer {
        from: ctx.accounts.token_vault_to_reclaim_from.to_account_info(),
        to: ctx.accounts.destination_token_account.to_account_info(),
        authority: ctx.accounts.admin.to_account_info(),
    };

    let campaign_seeds = &[
        CAMPAIGN_V0_SEED_PREFIX,
        campaign_fingerprint.as_ref(),
        &[ctx.accounts.campaign.bump],
    ];
    let signer_seeds = &[&campaign_seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        ),
        amount_to_reclaim,
    )?;

    // Optionally, emit an event
    // emit!(FundsWithdrawn {
    //     campaign_pda: ctx.accounts.campaign.key(),
    //     cohort_pda: ctx.accounts.cohort.key(),
    //     withdrawn_from_vault: ctx.accounts.token_vault_to_withdraw_from.key(),
    //     withdrawn_to_account: ctx.accounts.destination_token_account.key(),
    //     amount_withdrawn: amount_to_withdraw,
    //     timestamp: Clock::get()?.unix_timestamp,
    // });

    Ok(())
}

// Optional Event:
// #[event]
// pub struct FundsWithdrawn {
//     pub campaign_pda: Pubkey,
//     pub cohort_pda: Pubkey,
//     pub withdrawn_from_vault: Pubkey,
//     pub withdrawn_to_account: Pubkey,
//     pub amount_withdrawn: u64,
//     pub timestamp: i64,
// }

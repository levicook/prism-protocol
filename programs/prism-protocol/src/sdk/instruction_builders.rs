use anchor_lang::solana_program::{
    instruction::Instruction, system_program::ID as SYSTEM_PROGRAM_ID, sysvar,
};
use anchor_lang::{prelude::*, InstructionData as _};

use crate::ID as PRISM_PROGRAM_ID;

pub fn build_initialize_campaign_ix(
    admin: Pubkey,
    campaign: Pubkey,
    campaign_fingerprint: [u8; 32],
    mint: Pubkey,
) -> Result<(
    Instruction,
    crate::accounts::InitializeCampaignV0,
    crate::instruction::InitializeCampaignV0,
)> {
    let ix_accounts = crate::accounts::InitializeCampaignV0 {
        admin,
        campaign,
        system_program: SYSTEM_PROGRAM_ID,
    };

    let ix_data = crate::instruction::InitializeCampaignV0 {
        campaign_fingerprint,
        mint,
    };

    let ix = Instruction {
        program_id: PRISM_PROGRAM_ID,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_initialize_cohort_ix(
    admin: Pubkey,
    campaign: Pubkey,
    campaign_fingerprint: [u8; 32],
    cohort: Pubkey,
    merkle_root: [u8; 32],
    amount_per_entitlement: u64,
    vaults: Vec<Pubkey>,
) -> Result<(
    Instruction,
    crate::accounts::InitializeCohortV0,
    crate::instruction::InitializeCohortV0,
)> {
    let ix_accounts = crate::accounts::InitializeCohortV0 {
        admin,
        campaign,
        cohort,
        system_program: SYSTEM_PROGRAM_ID,
    };

    let ix_data = crate::instruction::InitializeCohortV0 {
        campaign_fingerprint,
        merkle_root,
        amount_per_entitlement,
        vaults,
    };

    let ix = Instruction {
        program_id: PRISM_PROGRAM_ID,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_claim_tokens_ix(
    admin: Pubkey,
    claimant: Pubkey,
    campaign: Pubkey,
    cohort: Pubkey,
    token_vault: Pubkey,
    mint: Pubkey,
    claimant_token_account: Pubkey,
    claim_receipt: Pubkey,
    campaign_fingerprint: [u8; 32],
    cohort_merkle_root_arg: [u8; 32],
    merkle_proof: Vec<[u8; 32]>,
    assigned_vault_from_leaf: Pubkey,
    entitlements_from_leaf: u64,
) -> Result<(
    Instruction,
    crate::accounts::ClaimTokensV0,
    crate::instruction::ClaimTokensV0,
)> {
    let ix_accounts = crate::accounts::ClaimTokensV0 {
        admin,
        claimant,
        campaign,
        cohort,
        token_vault,
        mint,
        claimant_token_account,
        claim_receipt,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        system_program: SYSTEM_PROGRAM_ID,
        rent: sysvar::rent::ID,
    };

    let ix_data = crate::instruction::ClaimTokensV0 {
        campaign_fingerprint,
        cohort_merkle_root_arg,
        merkle_proof,
        assigned_vault_from_leaf,
        entitlements_from_leaf,
    };

    let ix = Instruction {
        program_id: PRISM_PROGRAM_ID,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

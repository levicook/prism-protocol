use anchor_lang::solana_program::{
    instruction::Instruction, system_program::ID as SYSTEM_PROGRAM_ID, sysvar,
};
use anchor_lang::{prelude::*, InstructionData as _};
use prism_protocol::ID as PRISM_PROGRAM_ID;

pub fn build_initialize_campaign_ix(
    admin: Pubkey,
    campaign: Pubkey,
    campaign_fingerprint: [u8; 32],
    mint: Pubkey,
) -> Result<(
    Instruction,
    prism_protocol::accounts::InitializeCampaignV0,
    prism_protocol::instruction::InitializeCampaignV0,
)> {
    let ix_accounts = prism_protocol::accounts::InitializeCampaignV0 {
        admin,
        campaign,
        system_program: SYSTEM_PROGRAM_ID,
    };

    let ix_data = prism_protocol::instruction::InitializeCampaignV0 {
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
    vault_count: u8,
) -> Result<(
    Instruction,
    prism_protocol::accounts::InitializeCohortV0,
    prism_protocol::instruction::InitializeCohortV0,
)> {
    let ix_accounts = prism_protocol::accounts::InitializeCohortV0 {
        admin,
        campaign,
        cohort,
        system_program: SYSTEM_PROGRAM_ID,
    };

    let ix_data = prism_protocol::instruction::InitializeCohortV0 {
        campaign_fingerprint,
        merkle_root,
        amount_per_entitlement,
        vault_count,
    };

    let ix = Instruction {
        program_id: PRISM_PROGRAM_ID,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_create_vault_ix(
    admin: Pubkey,
    campaign: Pubkey,
    cohort: Pubkey,
    mint: Pubkey,
    vault: Pubkey,
    campaign_fingerprint: [u8; 32],
    cohort_merkle_root: [u8; 32],
    vault_index: u8,
) -> Result<(
    Instruction,
    prism_protocol::accounts::CreateVaultV0,
    prism_protocol::instruction::CreateVaultV0,
)> {
    let ix_accounts = prism_protocol::accounts::CreateVaultV0 {
        admin,
        campaign,
        cohort,
        mint,
        vault,
        token_program: anchor_spl::token::ID,
        system_program: SYSTEM_PROGRAM_ID,
    };

    let ix_data = prism_protocol::instruction::CreateVaultV0 {
        campaign_fingerprint,
        cohort_merkle_root,
        vault_index,
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
    vault: Pubkey,
    mint: Pubkey,
    claimant_token_account: Pubkey,
    claim_receipt: Pubkey,
    campaign_fingerprint: [u8; 32],
    cohort_merkle_root: [u8; 32],
    merkle_proof: Vec<[u8; 32]>,
    assigned_vault_index: u8,
    entitlements: u64,
) -> Result<(
    Instruction,
    prism_protocol::accounts::ClaimTokensV0,
    prism_protocol::instruction::ClaimTokensV0,
)> {
    let ix_accounts = prism_protocol::accounts::ClaimTokensV0 {
        admin,
        claimant,
        campaign,
        cohort,
        vault,
        mint,
        claimant_token_account,
        claim_receipt,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        system_program: SYSTEM_PROGRAM_ID,
        rent: sysvar::rent::ID,
    };

    let ix_data = prism_protocol::instruction::ClaimTokensV0 {
        campaign_fingerprint,
        cohort_merkle_root,
        merkle_proof,
        assigned_vault_index,
        entitlements,
    };

    let ix = Instruction {
        program_id: PRISM_PROGRAM_ID,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}
pub fn build_set_campaign_active_status_ix(
    admin: Pubkey,
    campaign: Pubkey,
    campaign_fingerprint: [u8; 32],
    is_active: bool,
) -> Result<(
    Instruction,
    prism_protocol::accounts::SetCampaignActiveStatus,
    prism_protocol::instruction::SetCampaignActiveStatus,
)> {
    let ix_accounts = prism_protocol::accounts::SetCampaignActiveStatus { admin, campaign };

    let ix_data = prism_protocol::instruction::SetCampaignActiveStatus {
        campaign_fingerprint,
        is_active,
    };

    let ix = Instruction {
        program_id: PRISM_PROGRAM_ID,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}


use crate::AddressFinder;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData as _, prelude::*};

pub fn build_initialize_campaign_v0_ix(
    address_finder: &AddressFinder,
    expected_cohort_count: u8,
) -> Result<(
    Instruction,
    prism_protocol::accounts::InitializeCampaignV0,
    prism_protocol::instruction::InitializeCampaignV0,
)> {
    let ix_accounts = prism_protocol::accounts::InitializeCampaignV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
        system_program: address_finder.system_program_id,
    };

    let ix_data = prism_protocol::instruction::InitializeCampaignV0 {
        mint: address_finder.mint,
        expected_cohort_count,
    };

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_activate_campaign_v0_ix(
    address_finder: &AddressFinder,
    final_db_ipfs_hash: [u8; 32],
    go_live_slot: u64,
) -> Result<(
    Instruction,
    prism_protocol::accounts::ActivateCampaignV0,
    prism_protocol::instruction::ActivateCampaignV0,
)> {
    let ix_accounts = prism_protocol::accounts::ActivateCampaignV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
    };

    let ix_data = prism_protocol::instruction::ActivateCampaignV0 {
        final_db_ipfs_hash,
        go_live_slot,
    };

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_make_campaign_unstoppable_v0_ix(
    address_finder: &AddressFinder,
) -> Result<(
    Instruction,
    prism_protocol::accounts::MakeCampaignUnstoppableV0,
    prism_protocol::instruction::MakeCampaignUnstoppableV0,
)> {
    let ix_accounts = prism_protocol::accounts::MakeCampaignUnstoppableV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
    };

    let ix_data = prism_protocol::instruction::MakeCampaignUnstoppableV0 {};

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_pause_campaign_v0_ix(
    address_finder: &AddressFinder,
) -> Result<(
    Instruction,
    prism_protocol::accounts::PauseCampaignV0,
    prism_protocol::instruction::PauseCampaignV0,
)> {
    let ix_accounts = prism_protocol::accounts::PauseCampaignV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
    };

    let ix_data = prism_protocol::instruction::PauseCampaignV0 {};

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_resume_campaign_v0_ix(
    address_finder: &AddressFinder,
) -> Result<(
    Instruction,
    prism_protocol::accounts::ResumeCampaignV0,
    prism_protocol::instruction::ResumeCampaignV0,
)> {
    let ix_accounts = prism_protocol::accounts::ResumeCampaignV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
    };

    let ix_data = prism_protocol::instruction::ResumeCampaignV0 {};

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_permanently_halt_campaign_v0_ix(
    address_finder: &AddressFinder,
) -> Result<(
    Instruction,
    prism_protocol::accounts::PermanentlyHaltCampaignV0,
    prism_protocol::instruction::PermanentlyHaltCampaignV0,
)> {
    let ix_accounts = prism_protocol::accounts::PermanentlyHaltCampaignV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
    };

    let ix_data = prism_protocol::instruction::PermanentlyHaltCampaignV0 {};

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_initialize_cohort_v0_ix(
    address_finder: &AddressFinder,
    merkle_root: [u8; 32],
    amount_per_entitlement: u64,
    expected_vault_count: u8,
) -> Result<(
    Instruction,
    prism_protocol::accounts::InitializeCohortV0,
    prism_protocol::instruction::InitializeCohortV0,
)> {
    let (cohort, _) = address_finder.find_cohort_v0_address(&merkle_root);

    let ix_accounts = prism_protocol::accounts::InitializeCohortV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
        cohort,
        system_program: address_finder.system_program_id,
    };

    let ix_data = prism_protocol::instruction::InitializeCohortV0 {
        merkle_root,
        amount_per_entitlement,
        expected_vault_count,
    };

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_activate_cohort_v0_ix(
    address_finder: &AddressFinder,
    cohort_merkle_root: [u8; 32],
) -> Result<(
    Instruction,
    prism_protocol::accounts::ActivateCohortV0,
    prism_protocol::instruction::ActivateCohortV0,
)> {
    let (cohort, _) = address_finder.find_cohort_v0_address(&cohort_merkle_root);

    let ix_accounts = prism_protocol::accounts::ActivateCohortV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
        cohort,
    };

    let ix_data = prism_protocol::instruction::ActivateCohortV0 { cohort_merkle_root };

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_initialize_vault_v0_ix(
    address_finder: &AddressFinder,
    cohort_merkle_root: [u8; 32],
    mint: Pubkey,
    vault_index: u8,
) -> Result<(
    Instruction,
    prism_protocol::accounts::InitializeVaultV0,
    prism_protocol::instruction::InitializeVaultV0,
)> {
    let (cohort, _) = address_finder.find_cohort_v0_address(&cohort_merkle_root);

    let (vault, _) = address_finder.find_vault_v0_address(&cohort, vault_index);

    let ix_accounts = prism_protocol::accounts::InitializeVaultV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
        cohort,
        mint: address_finder.mint,
        vault,
        token_program: address_finder.token_program_id,
        system_program: address_finder.system_program_id,
    };

    let ix_data = prism_protocol::instruction::InitializeVaultV0 {
        cohort_merkle_root,
        vault_index,
    };

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_activate_vault_v0_ix(
    address_finder: &AddressFinder,
    cohort_merkle_root: [u8; 32],
    vault_index: u8,
    expected_balance: u64,
) -> Result<(
    Instruction,
    prism_protocol::accounts::ActivateVaultV0,
    prism_protocol::instruction::ActivateVaultV0,
)> {
    let (cohort, _) = address_finder.find_cohort_v0_address(&cohort_merkle_root);

    let (vault, _) = address_finder.find_vault_v0_address(&cohort, vault_index);

    let ix_accounts = prism_protocol::accounts::ActivateVaultV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
        cohort,
        vault,
    };

    let ix_data = prism_protocol::instruction::ActivateVaultV0 {
        cohort_merkle_root,
        vault_index,
        expected_balance,
    };

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_claim_tokens_v0_ix(
    address_finder: &AddressFinder,
    claimant: Pubkey,
    claimant_token_account: Pubkey,
    cohort_merkle_root: [u8; 32],
    merkle_proof: Vec<[u8; 32]>,
    assigned_vault_index: u8,
    entitlements: u64,
) -> Result<(
    Instruction,
    prism_protocol::accounts::ClaimTokensV0,
    prism_protocol::instruction::ClaimTokensV0,
)> {
    let (cohort, _) = address_finder.find_cohort_v0_address(&cohort_merkle_root);

    let (vault, _) = address_finder.find_vault_v0_address(&cohort, assigned_vault_index);

    let (claim_receipt, _) = address_finder.find_claim_receipt_v0_address(&cohort, &claimant);

    let ix_accounts = prism_protocol::accounts::ClaimTokensV0 {
        admin: address_finder.admin,
        claimant,
        campaign: address_finder.campaign,
        cohort,
        vault,
        mint: address_finder.mint,
        claimant_token_account,
        claim_receipt,
        token_program: address_finder.token_program_id,
        associated_token_program: address_finder.associated_token_program_id,
        system_program: address_finder.system_program_id,
        rent: address_finder.rent_id,
    };

    let ix_data = prism_protocol::instruction::ClaimTokensV0 {
        cohort_merkle_root,
        merkle_proof,
        assigned_vault_index,
        entitlements,
    };

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

pub fn build_reclaim_tokens_v0_ix(
    address_finder: &AddressFinder,
    destination_token_account: Pubkey,
    cohort_merkle_root_arg: [u8; 32],
    vault_index: u8,
) -> Result<(
    Instruction,
    prism_protocol::accounts::ReclaimTokensV0,
    prism_protocol::instruction::ReclaimTokensV0,
)> {
    let (cohort, _) = address_finder.find_cohort_v0_address(&cohort_merkle_root_arg);

    let (vault, _) = address_finder.find_vault_v0_address(&cohort, vault_index);

    let ix_accounts = prism_protocol::accounts::ReclaimTokensV0 {
        admin: address_finder.admin,
        campaign: address_finder.campaign,
        cohort,
        vault,
        destination_token_account,
        token_program: address_finder.token_program_id,
    };

    let ix_data = prism_protocol::instruction::ReclaimTokensV0 {
        cohort_merkle_root_arg,
        vault_index,
    };

    let ix = Instruction {
        program_id: address_finder.prism_program_id,
        accounts: ix_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    Ok((ix, ix_accounts, ix_data))
}

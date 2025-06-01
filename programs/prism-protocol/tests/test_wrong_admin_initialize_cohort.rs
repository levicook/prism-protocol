#![cfg(feature = "test-sbf")]

use {
    anchor_lang::InstructionData,
    mollusk_svm::result::Check,
    prism_protocol::error::ErrorCode as PrismError,
    prism_protocol_testing::{CampaignLifecycleStage, TestFixture},
    solana_sdk::{
        account::Account as SolanaAccount,
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        signature::{Keypair, Signer},
        system_program::ID as SYSTEM_PROGRAM_ID,
    },
};

#[test]
#[ignore = "this can't be tested with Mollusk"]
fn test_wrong_admin_initialize_cohort() {
    let mut fixture = TestFixture::new();
    let state = fixture.setup_to_stage(CampaignLifecycleStage::CampaignInitialized);

    let (valid_cohort, _) = fixture.address_finder.find_cohort_v0_address(
        &state.campaign.address, //
        &fixture.test_fingerprint,
    );

    let valid_ix_data = prism_protocol::instruction::InitializeCohortV0 {
        campaign_fingerprint: fixture.test_fingerprint,
        merkle_root: [1u8; 32],
        amount_per_entitlement: 1_000_000_000,
        expected_vault_count: 1,
    };

    // Prepare the wrong admin
    let invalid_admin = Keypair::new();

    // Manually build an invalid instruction
    let invalid_ix_accounts = vec![
        AccountMeta::new_readonly(fixture.address_finder.system_program_id, false),
        // AccountMeta::new(fixture.admin_address, true), // correct admin -- claim they're a signer
        AccountMeta::new(invalid_admin.pubkey(), true), // wrong admin -- also claim they're a signer
        AccountMeta::new(state.campaign.address, false), // correct campaign
        AccountMeta::new(valid_cohort, false),          // correct cohort
    ];

    let invalid_ix = Instruction {
        program_id: fixture.address_finder.program_id,
        accounts: invalid_ix_accounts,
        data: valid_ix_data.data(),
    };

    // let keyed_account_for_correct_admin = (
    //     fixture.admin_address,
    //     SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
    // );

    let keyed_account_for_wrong_admin = (
        invalid_admin.pubkey(),
        SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
    );

    let keyed_account_for_campaign = (
        state.campaign.address,
        SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
    );

    let keyed_account_for_cohort = (valid_cohort, SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID));

    // This should fail with CampaignAdminMismatch
    let _result = fixture.mollusk.process_and_validate_instruction(
        &invalid_ix,
        &[
            mollusk_svm::program::keyed_account_for_system_program(),
            // keyed_account_for_correct_admin,
            keyed_account_for_wrong_admin,
            keyed_account_for_campaign,
            keyed_account_for_cohort,
        ],
        &[Check::err(ProgramError::Custom(
            PrismError::CampaignAdminMismatch as u32,
        ))],
    );
}

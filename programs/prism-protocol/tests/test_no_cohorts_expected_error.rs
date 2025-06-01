#![cfg(feature = "test-sbf")]

use {
    mollusk_svm::{program::keyed_account_for_system_program, result::Check},
    prism_protocol::error::ErrorCode as PrismError,
    prism_protocol_sdk::build_initialize_campaign_v0_ix,
    prism_protocol_testing::{generate_test_fingerprint, TestFixture},
    solana_sdk::{
        account::Account as SolanaAccount,
        program_error::ProgramError,
        signature::{Keypair, Signer},
        system_program::ID as SYSTEM_PROGRAM_ID,
    },
};

#[test]
fn test_no_cohorts_expected_error() {
    let fixture = TestFixture::new();

    // Create test mint
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();

    let test_fingerprint = generate_test_fingerprint("test_no_cohorts_expected_error");

    // Try to initialize campaign with 0 expected cohorts (should fail)
    let (initialize_campaign_ix, ix_accounts, _) = build_initialize_campaign_v0_ix(
        &fixture.address_finder,
        fixture.admin_address,
        test_fingerprint,
        mint,
        0, // expected_cohort_count = 0 (invalid)
    )
    .expect("Failed to build initialize_campaign instruction");

    let keyed_account_for_admin = (
        fixture.admin_address,
        SolanaAccount::new(1_000_000_000, 0, &SYSTEM_PROGRAM_ID),
    );

    let keyed_account_for_campaign = (
        ix_accounts.campaign,
        SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
    );

    // This should fail with NoCohortsExpected error
    let result = fixture.mollusk.process_and_validate_instruction(
        &initialize_campaign_ix,
        &[
            keyed_account_for_system_program(),
            keyed_account_for_admin,
            keyed_account_for_campaign,
        ],
        &[Check::err(ProgramError::Custom(
            PrismError::NoCohortsExpected as u32,
        ))],
    );

    println!("✅ No cohorts expected error correctly triggered");
}

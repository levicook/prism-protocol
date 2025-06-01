#![cfg(feature = "test-sbf")]

use {
    mollusk_svm::result::Check,
    prism_protocol_sdk::build_initialize_cohort_v0_ix,
    prism_protocol_testing::{generate_test_fingerprint, TestFixture},
    solana_sdk::{
        account::Account as SolanaAccount,
        program_error::ProgramError,
        signature::{Keypair, Signer},
        system_program::ID as SYSTEM_PROGRAM_ID,
    },
};

#[test]
fn test_admin_mismatch_error_message() {
    let mut fixture = TestFixture::new();

    // Set up a campaign
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();

    let campaign = fixture.initialize_campaign_v0(mint, 1);

    // Now try to create a cohort with a completely different signature
    let different_signer = Keypair::new();
    let test_fingerprint = generate_test_fingerprint("test_admin_mismatch_error_message");
    let merkle_root = [2u8; 32];

    // Build instruction with correct admin address but will sign with different signer
    let (initialize_cohort_ix, ix_accounts, _) = build_initialize_cohort_v0_ix(
        &fixture.address_finder,
        fixture.admin_address, // Correct admin address
        test_fingerprint,
        merkle_root,
        1_000_000_000,
        1,
    )
    .expect("Failed to build initialize_cohort instruction");

    // Create modified instruction that uses different_signer instead of fixture.admin_keypair
    let mut modified_ix = initialize_cohort_ix.clone();
    modified_ix.accounts[1].pubkey = different_signer.pubkey(); // Change admin account

    let keyed_account_for_different_signer = (
        different_signer.pubkey(),
        SolanaAccount::new(1_000_000_000, 0, &SYSTEM_PROGRAM_ID),
    );

    let keyed_account_for_campaign = (campaign.address, campaign.campaign_account.clone());

    let keyed_account_for_cohort = (
        ix_accounts.cohort, // Extract pubkey from accounts struct
        SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
    );

    // This should fail with constraint or admin mismatch error
    let result = fixture.mollusk.process_and_validate_instruction(
        &modified_ix,
        &[
            mollusk_svm::program::keyed_account_for_system_program(),
            keyed_account_for_different_signer,
            keyed_account_for_campaign,
            keyed_account_for_cohort,
        ],
        &[Check::err(ProgramError::Custom(1))], // Just check that it fails
    );

    println!("✅ Admin mismatch correctly detected and failed");
}

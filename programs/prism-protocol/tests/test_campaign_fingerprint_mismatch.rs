#![cfg(feature = "test-sbf")]

use {
    mollusk_svm::{program::keyed_account_for_system_program, result::Check},
    prism_protocol::error::ErrorCode as PrismError,
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
fn test_campaign_fingerprint_mismatch() {
    let mut fixture = TestFixture::new();

    // Create test mint
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();

    // Initialize campaign with one fingerprint
    let campaign = fixture.initialize_campaign_v0(mint, 1);

    // Now try to create a cohort with a DIFFERENT fingerprint (should fail)
    let different_fingerprint = generate_test_fingerprint("different_fingerprint");
    let merkle_root = [1u8; 32];

    let (initialize_cohort_ix, ix_accounts, _) = build_initialize_cohort_v0_ix(
        &fixture.address_finder,
        fixture.admin_address,
        different_fingerprint, // Different fingerprint!
        merkle_root,
        1_000_000_000,
        1,
    )
    .expect("Failed to build initialize_cohort instruction");

    let keyed_account_for_admin = (fixture.admin_address, campaign.admin_account.clone());

    let keyed_account_for_campaign = (campaign.address, campaign.campaign_account.clone());

    let keyed_account_for_cohort = (
        ix_accounts.cohort,
        SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
    );

    // This should fail with CampaignFingerprintMismatch or constraint error
    let result = fixture.mollusk.process_and_validate_instruction(
        &initialize_cohort_ix,
        &[
            keyed_account_for_system_program(),
            keyed_account_for_admin,
            keyed_account_for_campaign,
            keyed_account_for_cohort,
        ],
        &[Check::err(ProgramError::Custom(
            PrismError::CampaignFingerprintMismatch as u32,
        ))],
    );

    println!("✅ Campaign fingerprint mismatch correctly detected");
}

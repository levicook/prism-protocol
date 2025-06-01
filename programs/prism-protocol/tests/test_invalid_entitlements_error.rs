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
fn test_invalid_entitlements_error() {
    let mut fixture = TestFixture::new();

    // Set up campaign first
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();

    let campaign = fixture.initialize_campaign_v0(mint, 1);

    let test_fingerprint = generate_test_fingerprint("test_invalid_entitlements_error");
    let merkle_root = [1u8; 32];

    // Try to initialize cohort with 0 amount per entitlement (should fail)
    let (initialize_cohort_ix, ix_accounts, _) = build_initialize_cohort_v0_ix(
        &fixture.address_finder,
        fixture.admin_address,
        test_fingerprint,
        merkle_root,
        0, // amount_per_entitlement = 0 (invalid)
        1, // expected_vault_count
    )
    .expect("Failed to build initialize_cohort instruction");

    let keyed_account_for_admin = (fixture.admin_address, campaign.admin_account.clone());

    let keyed_account_for_campaign = (campaign.address, campaign.campaign_account.clone());

    let keyed_account_for_cohort = (
        ix_accounts.cohort,
        SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
    );

    // This should fail with InvalidEntitlements error
    let result = fixture.mollusk.process_and_validate_instruction(
        &initialize_cohort_ix,
        &[
            keyed_account_for_system_program(),
            keyed_account_for_admin,
            keyed_account_for_campaign,
            keyed_account_for_cohort,
        ],
        &[Check::err(ProgramError::Custom(
            PrismError::InvalidEntitlements as u32,
        ))],
    );

    println!("✅ Invalid entitlements error correctly triggered");
}

#![cfg(feature = "test-sbf")]

use {
    mollusk_svm::result::Check,
    prism_protocol::error::ErrorCode as PrismError,
    prism_protocol_sdk::build_initialize_vault_v0_ix,
    prism_protocol_testing::{generate_test_fingerprint, TestFixture},
    solana_sdk::{
        account::Account as SolanaAccount,
        program_error::ProgramError,
        signature::{Keypair, Signer},
        system_program::ID as SYSTEM_PROGRAM_ID,
    },
};

#[test]
fn test_wrong_admin_initialize_vault() {
    let mut fixture = TestFixture::new();

    // Set up campaign and cohort first
    let mint_keypair = Keypair::new();
    let mint_account = fixture.create_mint(&mint_keypair, 9);
    let mint = mint_keypair.pubkey();

    let campaign = fixture.initialize_campaign_v0(mint, 1);
    let claimants = vec![Keypair::new().pubkey(), Keypair::new().pubkey()];
    let cohort = fixture.initialize_cohort_v0(
        &campaign,
        &claimants,
        1,             // expected_vault_count
        1_000_000_000, // amount_per_entitlement
    );

    // Create a different admin
    let wrong_admin = Keypair::new();
    let test_fingerprint = generate_test_fingerprint("test_wrong_admin_initialize_vault");
    let merkle_root = cohort
        .merkle_tree
        .root()
        .expect("Failed to get merkle root");

    // Try to initialize vault with wrong admin
    let (initialize_vault_ix, ix_accounts, _) = build_initialize_vault_v0_ix(
        &fixture.address_finder,
        wrong_admin.pubkey(), // Wrong admin
        test_fingerprint,
        merkle_root,
        mint,
        0, // vault_index
    )
    .expect("Failed to build initialize_vault instruction");

    let keyed_account_for_wrong_admin = (wrong_admin.pubkey(), campaign.admin_account.clone());

    let keyed_account_for_campaign = (campaign.address, campaign.campaign_account.clone());

    let keyed_account_for_cohort = (
        ix_accounts.cohort, // Extract pubkey from accounts struct
        cohort.cohort_account.clone(),
    );

    let keyed_account_for_mint = (mint, mint_account);

    let keyed_account_for_vault = (
        ix_accounts.vault, // Extract pubkey from accounts struct
        SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
    );

    // This should fail with constraint error
    let result = fixture.mollusk.process_and_validate_instruction(
        &initialize_vault_ix,
        &[
            mollusk_svm::program::keyed_account_for_system_program(),
            keyed_account_for_wrong_admin,
            keyed_account_for_campaign,
            keyed_account_for_cohort,
            keyed_account_for_mint,
            keyed_account_for_vault,
            mollusk_svm_programs_token::token::keyed_account(),
        ],
        &[Check::err(ProgramError::Custom(
            PrismError::CampaignAdminMismatch as u32,
        ))],
    );

    println!("✅ Wrong admin vault initialization correctly failed");
}

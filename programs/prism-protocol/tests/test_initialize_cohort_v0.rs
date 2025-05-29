#![cfg(feature = "test-sbf")]

use {
    anchor_lang::prelude::{AccountDeserialize, Pubkey},
    prism_protocol::state::CohortV0,
    prism_protocol_testing::{TestFixture, TEST_AMOUNT_PER_ENTITLEMENT},
    solana_sdk::signature::{Keypair, Signer},
};

#[test]
fn test_initialize_cohort_v0() {
    let mut fixture = TestFixture::new();

    // Create a test mint
    let mint_keypair = Keypair::new();
    let _mint_account = fixture.create_mint(&mint_keypair, 9);
    let mint = mint_keypair.pubkey();

    let campaign_result = fixture.initialize_campaign(mint);

    // Setup test data
    let amount_per_entitlement = TEST_AMOUNT_PER_ENTITLEMENT;
    let vault_count = 3;

    // Initialize cohort
    let cohort_result = fixture.initialize_cohort_with_merkle_tree(
        &campaign_result,
        &[Pubkey::new_unique(), Pubkey::new_unique()], // claimants
        vault_count,
        amount_per_entitlement,
    );

    // Verify the cohort was created correctly
    let cohort_account = &cohort_result.cohort_account;
    assert_eq!(cohort_account.owner, prism_protocol::ID);

    // Deserialize and verify the cohort state
    let cohort_state = CohortV0::try_deserialize(&mut cohort_account.data.as_slice())
        .expect("Failed to deserialize cohort state");

    assert_eq!(cohort_state.campaign, campaign_result.address);
    assert_eq!(cohort_state.amount_per_entitlement, amount_per_entitlement);
    assert_eq!(cohort_state.bump, cohort_result.bump);
    assert_eq!(cohort_state.vaults.len(), vault_count);

    // Verify all vaults are initialized to default (empty) pubkeys
    for vault in &cohort_state.vaults {
        assert_eq!(*vault, Pubkey::default());
    }

    println!("✅ Cohort initialized successfully");
    println!("   - Address: {}", cohort_result.address);
    println!("   - Campaign: {}", cohort_state.campaign);
    println!(
        "   - Amount per entitlement: {}",
        cohort_state.amount_per_entitlement
    );
    println!("   - Vault count: {}", cohort_state.vaults.len());
}

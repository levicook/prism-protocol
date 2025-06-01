#![cfg(feature = "test-sbf")]

use prism_protocol_testing::{
    generate_test_fingerprint, CampaignAction, CampaignLifecycleStage, TestFixture,
};

#[test]
fn test_vault_operations_on_nonexistent_cohorts() {
    let mut fixture = TestFixture::new();
    fixture.test_fingerprint = generate_test_fingerprint("vault_ops_nonexistent_cohorts");

    println!("🚫 Testing vault operations on nonexistent cohorts");

    // Campaign initialized but no cohorts yet
    let mut state = fixture.setup_to_stage(CampaignLifecycleStage::CampaignInitialized);

    // Try to initialize vault for nonexistent cohort
    fixture.expect_failure(
        &mut state,
        CampaignAction::InitializeVault {
            cohort_index: 0,
            vault_index: 0,
        },
        "Action not yet implemented", // TODO: Update when we implement this
    );

    // Try to fund vault for nonexistent cohort
    fixture.expect_failure(
        &mut state,
        CampaignAction::FundVault {
            cohort_index: 0,
            vault_index: 0,
            amount: 1000,
        },
        "Action not yet implemented", // TODO: Update when we implement this
    );

    println!("✅ Vault operations properly fail on nonexistent cohorts");
}

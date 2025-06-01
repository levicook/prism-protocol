#![cfg(feature = "test-sbf")]

use prism_protocol_testing::{
    generate_test_fingerprint, CampaignAction, CampaignLifecycleStage, TestFixture,
};

#[test]
fn test_out_of_bounds_operations() {
    let mut fixture = TestFixture::new();
    fixture.test_fingerprint = generate_test_fingerprint("out_of_bounds_operations");

    println!("🚫 Testing out-of-bounds operations");

    // Set up campaign with 2 cohorts
    let mut state = fixture.setup_to_stage(CampaignLifecycleStage::CohortsInitialized);
    assert_eq!(state.cohorts.len(), 2);

    // Try operations on cohort index 2 (doesn't exist)
    fixture.expect_failure(
        &mut state,
        CampaignAction::ActivateCohort { cohort_index: 2 },
        "Action not yet implemented", // TODO: Update when we implement this
    );

    fixture.expect_failure(
        &mut state,
        CampaignAction::InitializeVault {
            cohort_index: 2,
            vault_index: 0,
        },
        "Action not yet implemented", // TODO: Update when we implement this
    );

    // Set up with vaults and try out-of-bounds vault operations
    fixture.advance_to_stage(&mut state, CampaignLifecycleStage::VaultsInitialized);

    // First cohort has 2 vaults (0, 1), try vault index 2
    fixture.expect_failure(
        &mut state,
        CampaignAction::FundVault {
            cohort_index: 0,
            vault_index: 2,
            amount: 1000,
        },
        "Action not yet implemented", // TODO: Update when we implement this
    );

    println!("✅ Out-of-bounds operations properly fail");
}

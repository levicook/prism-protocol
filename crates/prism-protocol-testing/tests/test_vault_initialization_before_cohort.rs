use prism_protocol_testing::{demand_account_not_initialized_error, FixtureStage, TestFixture};

/// Test vault initialization before cohort (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign only
/// - Attempt to initialize vault WITHOUT cohort existing
/// - Verify operation fails (vault requires cohort to exist first)
/// - Ensure proper order dependencies are enforced
#[test]
fn test_vault_initialization_before_cohort() {
    let mut test = TestFixture::default();

    test.jump_to(FixtureStage::CampaignInitialized);

    let result = test.try_initialize_vaults();

    demand_account_not_initialized_error(result);

    println!("✅ Correctly prevented vault initialization before cohort");
}

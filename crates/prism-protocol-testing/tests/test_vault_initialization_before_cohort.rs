use prism_protocol_testing::{FixtureStage, TestFixture};

/// Test vault initialization before cohort (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign only
/// - Attempt to initialize vault WITHOUT cohort existing
/// - Verify operation fails (vault requires cohort to exist first)
/// - Ensure proper order dependencies are enforced
#[test]
#[ignore = "Test may need review - unclear if this order dependency actually exists in protocol"]
fn test_vault_initialization_before_cohort() {
    let mut test = TestFixture::default();

    // Setup campaign but not cohort
    test.jump_to(FixtureStage::CampaignInitialized)
        .expect("campaign initialization failed");

    // Try to initialize vault without cohort
    let result = test.jump_to(FixtureStage::VaultInitialized);

    // This should fail because no cohort exists yet
    assert!(
        result.is_err(),
        "Expected vault initialization to fail without cohort"
    );
    println!("✅ Correctly prevented premature vault initialization");
}

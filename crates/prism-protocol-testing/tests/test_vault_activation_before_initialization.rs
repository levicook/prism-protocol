use prism_protocol_testing::{FixtureStage, TestFixture};

/// Test vault activation before initialization (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign and cohort
/// - Attempt to activate vault WITHOUT initializing vault first
/// - Verify operation fails (activation requires initialization)
/// - Ensure proper order dependencies are enforced
#[test]
#[ignore = "Test may need review - unclear if this order dependency actually exists in protocol"]
fn test_vault_activation_before_initialization() {
    let mut test = TestFixture::default();

    // Setup campaign and cohort but not vault
    test.jump_to(FixtureStage::CampaignInitialized)
        .expect("campaign initialization failed");

    test.jump_to(FixtureStage::CohortsInitialized)
        .expect("cohort initialization failed");

    // Try to activate vault without initializing it first
    let result = test.jump_to(FixtureStage::VaultsActivated);

    // This should fail because vault isn't initialized yet
    assert!(
        result.is_err(),
        "Expected vault activation to fail without vault initialization"
    );
    println!("✅ Correctly prevented premature vault activation");
}

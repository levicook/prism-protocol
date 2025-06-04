use prism_protocol_testing::{demand_account_not_initialized_error, FixtureStage, TestFixture};

/// Test vault activation before initialization (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign and cohort
/// - Attempt to activate vault WITHOUT initializing vault first
/// - Verify operation fails with AccountNotInitialized error code
/// - Ensure proper order dependencies are enforced
#[test]
fn test_vault_activation_before_initialization() {
    let mut test = TestFixture::default();

    test.jump_to(FixtureStage::CohortsInitialized);

    let result = test.try_activate_vaults();

    demand_account_not_initialized_error(result);

    println!("✅ Correctly prevented vault activation before initialization");
}

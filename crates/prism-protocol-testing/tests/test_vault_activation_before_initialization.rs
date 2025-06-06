use litesvm::LiteSVM;
use prism_protocol_testing::{
    demand_account_not_initialized_error, FixtureStage, FixtureState, TestFixture,
};

/// Test vault activation before initialization (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign and cohort
/// - Attempt to activate vault WITHOUT initializing vault first
/// - Verify operation fails with AccountNotInitialized error code
/// - Ensure proper order dependencies are enforced
#[tokio::test]
async fn test_vault_activation_before_initialization() {
    let state = FixtureState::rand().await;
    let mut test = TestFixture::new(state, LiteSVM::new())
        .await
        .expect("Failed to create test fixture");

    test.jump_to(FixtureStage::CohortsInitialized).await;

    let result = test.try_activate_vaults().await;

    demand_account_not_initialized_error(result);

    println!("✅ Correctly prevented vault activation before initialization");
}

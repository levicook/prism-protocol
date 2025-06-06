use litesvm::LiteSVM;
use prism_protocol_testing::{demand_account_not_initialized_error, FixtureStage, FixtureState, TestFixture};

/// Test vault initialization before cohort (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign only
/// - Attempt to initialize vault WITHOUT cohort existing
/// - Verify operation fails (vault requires cohort to exist first)
/// - Ensure proper order dependencies are enforced
#[tokio::test]
async fn test_vault_initialization_before_cohort() {
    let state = FixtureState::new().await;
    let mut test = TestFixture::new(state, LiteSVM::new())
        .await
        .expect("Failed to create test fixture");

    test.jump_to(FixtureStage::CampaignInitialized).await;

    let result = test.try_initialize_vaults().await;

    demand_account_not_initialized_error(result);

    println!("✅ Correctly prevented vault initialization before cohort");
}

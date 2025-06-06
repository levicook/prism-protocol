use litesvm::LiteSVM;
use prism_protocol_testing::{
    demand_invalid_account_data_error, FixtureStage, FixtureState, TestFixture,
};

/// Test vault funding before initialization (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign and cohorts only
/// - Attempt to fund vaults WITHOUT vaults being initialized first
/// - Verify operation fails (vault funding requires vault to exist first)
/// - Ensure proper order dependencies are enforced
#[tokio::test]
async fn test_vault_funding_before_initialization() {
    let state = FixtureState::rand().await;
    let mut test = TestFixture::new(state, LiteSVM::new())
        .await
        .expect("Failed to create test fixture");

    test.jump_to(FixtureStage::CohortsInitialized).await;

    let result = test.try_fund_vaults().await;

    demand_invalid_account_data_error(result);

    println!("✅ Correctly prevented vault funding before initialization");
}

use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};

/// Test cohort activation before vault activation (wrong order) - should fail
///
/// Should test:
/// - Initialize and fund vaults but do not activate them
/// - Attempt to activate cohorts WITHOUT all vaults being activated first
/// - Verify operation fails with NotAllVaultsActivated error
/// - Ensure proper order dependencies are enforced
#[tokio::test]
async fn test_cohort_activation_before_vault_activation() {
    let state = FixtureState::rand().await;
    let mut test = TestFixture::new(state, LiteSVM::new()).await.unwrap();

    test.jump_to(FixtureStage::VaultsFunded).await;

    let result = test.try_activate_cohorts().await;

    demand_prism_error(
        result,
        PrismError::NotAllVaultsActivated as u32,
        "NotAllVaultsActivated",
    );

    println!("✅ Correctly prevented cohort activation before vault activation");
}

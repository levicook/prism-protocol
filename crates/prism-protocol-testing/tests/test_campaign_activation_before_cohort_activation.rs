use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};

/// Test campaign activation before cohort activation (wrong order) - should fail
///
/// Should test:
/// - Complete vault lifecycle but do not activate cohorts
/// - Attempt to activate campaign WITHOUT all cohorts being activated first
/// - Verify operation fails with NotAllCohortsActivated error
/// - Ensure proper order dependencies are enforced
#[tokio::test]
async fn test_campaign_activation_before_cohort_activation() {
    let state = FixtureState::rand().await;
    let mut test = TestFixture::new(state, LiteSVM::new())
        .await
        .expect("Failed to create test fixture");

    test.jump_to(FixtureStage::VaultsActivated).await;

    let result = test.try_activate_campaign().await;

    demand_prism_error(
        result,
        PrismError::NotAllCohortsActivated as u32,
        "NotAllCohortsActivated",
    );

    println!("✅ Correctly prevented campaign activation before cohort activation");
}

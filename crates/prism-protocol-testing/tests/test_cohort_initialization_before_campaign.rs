use litesvm::LiteSVM;
use prism_protocol_testing::{demand_account_not_initialized_error, FixtureStage, FixtureState, TestFixture};

/// Test cohort initialization before campaign (wrong order) - should fail
///
/// Should test:
/// - Start from compiled campaign stage (no campaign initialized yet)
/// - Attempt to initialize cohorts WITHOUT campaign existing
/// - Verify operation fails (cohorts require campaign to exist first)
/// - Ensure proper order dependencies are enforced
#[tokio::test]
async fn test_cohort_initialization_before_campaign() {
    let state = FixtureState::new().await;
    let mut test = TestFixture::new(state, LiteSVM::new())
        .await
        .expect("Failed to create test fixture");

    test.jump_to(FixtureStage::CampaignCompiled).await;

    let result = test.try_initialize_cohorts().await;

    demand_account_not_initialized_error(result);

    println!("✅ Correctly prevented cohort initialization before campaign");
}

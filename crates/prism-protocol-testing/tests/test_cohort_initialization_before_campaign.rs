use prism_protocol_testing::{demand_account_not_initialized_error, FixtureStage, TestFixture};

/// Test cohort initialization before campaign (wrong order) - should fail
///
/// Should test:
/// - Start from compiled campaign stage (no campaign initialized yet)
/// - Attempt to initialize cohorts WITHOUT campaign existing
/// - Verify operation fails (cohorts require campaign to exist first)
/// - Ensure proper order dependencies are enforced
#[test]
fn test_cohort_initialization_before_campaign() {
    let mut test = TestFixture::default();

    test.jump_to(FixtureStage::CampaignCompiled);

    let result = test.try_initialize_cohorts();

    demand_account_not_initialized_error(result);

    println!("✅ Correctly prevented cohort initialization before campaign");
}

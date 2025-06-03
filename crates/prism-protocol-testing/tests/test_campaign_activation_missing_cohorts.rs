use prism_protocol_testing::TestFixture;

/// Test campaign activation with missing cohorts → NotAllCohortsActivated
///
/// Should test:
/// - Set up campaign with some cohorts activated but not all
/// - Attempt campaign activation
/// - Verify fails with NotAllCohortsActivated error
/// - Ensure proper validation of cohort activation requirements
#[test]
#[ignore]
fn test_campaign_activation_missing_cohorts() {
    let mut _test = TestFixture::default();

    todo!("Implement campaign activation with missing cohorts test - should fail with NotAllCohortsActivated");
}

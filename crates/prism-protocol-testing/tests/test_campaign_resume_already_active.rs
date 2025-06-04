use prism_protocol_testing::TestFixture;

/// Test resume already active campaign → CampaignNotPaused
///
/// Should test:
/// - Set up active campaign
/// - Attempt to resume the already active campaign
/// - Verify fails with CampaignNotPaused error
/// - Ensure only paused campaigns can be resumed
#[test]
#[ignore]
fn test_campaign_resume_already_active() {
    let mut _test = TestFixture::default();

    todo!("Implement resume active campaign test - should fail with CampaignNotPaused");
}

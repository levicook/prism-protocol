use prism_protocol_testing::TestFixture;

/// Test pause inactive campaign → CampaignNotActive
///
/// Should test:
/// - Set up inactive campaign (before activation)
/// - Attempt to pause the inactive campaign
/// - Verify fails with CampaignNotActive error
/// - Ensure only active campaigns can be paused
#[test]
#[ignore]
fn test_campaign_pause_inactive() {
    let mut _test = TestFixture::default();

    todo!("Implement pause inactive campaign test - should fail with CampaignNotActive");
}

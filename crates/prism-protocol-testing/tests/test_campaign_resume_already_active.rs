use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};

/// Test resume already active campaign → CampaignNotPaused
///
/// Should test:
/// - Set up active campaign (never paused)
/// - Attempt to resume the already active campaign
/// - Verify fails with CampaignNotPaused error
/// - Ensure only paused campaigns can be resumed
#[tokio::test]
async fn test_campaign_resume_already_active() {
    let mut test = TestFixture::new(FixtureState::rand().await, LiteSVM::new())
        .await
        .unwrap();

    // 1. Set up active campaign (never paused)
    test.jump_to(FixtureStage::CampaignActivated).await;

    // 2. Attempt to resume the already active campaign (should fail)
    let result = test.try_resume_campaign().await;

    // 3. Verify fails with CampaignNotPaused error
    demand_prism_error(
        result,
        PrismError::CampaignNotPaused as u32,
        "CampaignNotPaused",
    );

    println!("✅ Already active campaign correctly rejected resume attempt with CampaignNotPaused");
}

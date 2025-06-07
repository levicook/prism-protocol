use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};

/// Test pause inactive campaign → CampaignNotActive
///
/// Should test:
/// - Set up inactive campaign (before activation)
/// - Attempt to pause the inactive campaign
/// - Verify fails with CampaignNotActive error
/// - Ensure only active campaigns can be paused
#[tokio::test]
async fn test_campaign_pause_inactive() {
    let mut test = TestFixture::new(FixtureState::rand().await, LiteSVM::new())
        .await
        .unwrap();

    // 1. Set up campaign but STOP before activation (status = Inactive)
    test.jump_to(FixtureStage::VaultsActivated).await;

    // 2. Attempt to pause the inactive campaign (should fail)
    let result = test.try_pause_campaign().await;

    // 3. Verify fails with CampaignNotActive error
    demand_prism_error(
        result,
        PrismError::CampaignNotActive as u32,
        "CampaignNotActive",
    );

    println!("✅ Inactive campaign correctly rejected pause attempt with CampaignNotActive");
}

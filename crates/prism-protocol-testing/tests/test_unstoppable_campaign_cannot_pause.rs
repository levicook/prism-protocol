use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};

/// Test pause unstoppable campaign → CampaignIsUnstoppable
///
/// Should test:
/// - Set up active unstoppable campaign
/// - Attempt to pause the unstoppable campaign
/// - Verify fails with CampaignIsUnstoppable error
/// - Ensure unstoppable campaigns cannot be paused or halted
#[tokio::test]
async fn test_unstoppable_campaign_cannot_pause() {
    let mut test = TestFixture::new(FixtureState::rand().await, LiteSVM::new())
        .await
        .unwrap();

    // 1. Set up active campaign and make it unstoppable
    test.jump_to(FixtureStage::CampaignActivated).await;

    // Make the campaign unstoppable
    test.try_make_campaign_unstoppable()
        .await
        .expect("Should be able to make campaign unstoppable");

    // 2. Attempt to pause the unstoppable campaign (should fail)
    let result = test.try_pause_campaign().await;

    // 3. Verify fails with CampaignIsUnstoppable error
    demand_prism_error(
        result,
        PrismError::CampaignIsUnstoppable as u32,
        "CampaignIsUnstoppable",
    );

    println!("✅ Unstoppable campaign correctly rejected pause attempt with CampaignIsUnstoppable");
}

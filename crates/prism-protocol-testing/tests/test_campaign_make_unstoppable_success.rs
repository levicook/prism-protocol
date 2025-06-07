use litesvm::LiteSVM;
use prism_protocol_testing::{FixtureStage, FixtureState, TestFixture};

/// Test successful campaign make unstoppable operation
///
/// Should test:
/// - Set up active campaign
/// - Make the campaign unstoppable (should succeed)
/// - Verify campaign status is now unstoppable
/// - Verify admin authorization and proper state changes
#[tokio::test]
async fn test_campaign_make_unstoppable_success() {
    let mut test = TestFixture::new(FixtureState::rand().await, LiteSVM::new())
        .await
        .unwrap();

    // 1. Set up active campaign
    test.jump_to(FixtureStage::CampaignActivated).await;

    // 2. Make the campaign unstoppable (should succeed)
    test.try_make_campaign_unstoppable()
        .await
        .expect("Should be able to make campaign unstoppable");

    println!("✅ Campaign successfully made unstoppable");
}

use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};

/// Test token reclamation from active campaign → CampaignNotPermanentlyHalted
///
/// Should test:
/// - Set up active campaign (not halted)
/// - Attempt to reclaim tokens while campaign is still active
/// - Verify fails with CampaignNotPermanentlyHalted error
/// - Ensure token reclamation only works on permanently halted campaigns
#[tokio::test]
async fn test_reclaim_tokens_from_active_campaign() {
    let mut test = TestFixture::new(FixtureState::rand().await, LiteSVM::new())
        .await
        .unwrap();

    // 1. Set up active campaign (not halted)
    test.jump_to(FixtureStage::CampaignActivated).await;

    // 2. Attempt to reclaim tokens while campaign is still active (should fail)
    let result = test.try_reclaim_tokens().await;

    // 3. Verify fails with CampaignNotPermanentlyHalted error
    demand_prism_error(
        result,
        PrismError::CampaignNotPermanentlyHalted as u32,
        "CampaignNotPermanentlyHalted",
    );

    println!(
        "✅ Active campaign correctly rejected token reclamation with CampaignNotPermanentlyHalted"
    );
}

use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};

/// Test campaign activation with go_live_slot in past → GoLiveSlotInPast
///
/// This test validates that campaigns cannot be activated with go-live slots in the past:
/// - Verifies proper timing validation during campaign activation
/// - Ensures campaigns can only go live at current or future slots  
/// - Tests the GoLiveSlotInPast error is correctly triggered and returned
/// - Demonstrates real-world scenario where activation parameters are misconfigured
///
/// **Background**: Campaign activation sets the go_live_slot when claims become available.
/// Setting this to a past slot would be nonsensical - claims should only be available
/// from the activation point forward, never retroactively. The protocol correctly
/// prevents this temporal inconsistency.
///
/// **Business rule enforced**: `go_live_slot >= current_slot` during activation
#[tokio::test]
async fn test_campaign_activation_go_live_slot_in_past() {
    let state = FixtureState::simple_v1().await;
    let mut test = TestFixture::new(state, LiteSVM::new()).await.unwrap();

    // Set up campaign ready for activation (all cohorts activated)
    test.jump_to(FixtureStage::CohortsActivated).await;

    // Advance to a meaningful slot so we can have a truly past slot
    test.advance_slot_by(20);

    // Get current slot and calculate a past slot
    let current_slot = test.current_slot();
    let past_go_live_slot = current_slot - 10; // 10 slots in the past

    println!(
        "⏰ Current slot: {}, Attempted go-live slot: {} (past)",
        current_slot, past_go_live_slot
    );

    // Attempt to activate campaign with go_live_slot in the past
    let result = test
        .try_activate_campaign_with_args(
            Some([1u8; 32]),         // Valid IPFS hash
            Some(past_go_live_slot), // ← This should trigger GoLiveSlotInPast error
        )
        .await;

    // Verify fails with GoLiveSlotInPast error (code 6011)
    demand_prism_error(
        result,
        PrismError::GoLiveSlotInPast as u32,
        "GoLiveSlotInPast",
    );

    println!("✅ Campaign activation correctly rejected past go_live_slot");

    // Verify campaign is still in Inactive status (no side effects)
    let campaign_after = test
        .fetch_campaign_account()
        .expect("Campaign account should exist");

    // Should still be inactive with original values
    assert!(matches!(
        campaign_after.status,
        prism_protocol::CampaignStatus::Inactive
    ));
    assert_eq!(campaign_after.campaign_db_ipfs_hash, [0u8; 32]); // Still unset
    assert_eq!(campaign_after.go_live_slot, 0); // Still unset

    println!("✅ Verified no side effects - campaign remains inactive");

    // Demonstrate the fix: activation with current/future slot works
    let valid_go_live_slot = current_slot + 5; // 5 slots in the future

    println!(
        "🔧 Demonstrating fix: activating with future slot {}",
        valid_go_live_slot
    );

    test.try_activate_campaign_with_args(
        Some([1u8; 32]),          // Valid IPFS hash
        Some(valid_go_live_slot), // Valid future slot
    )
    .await
    .expect("Campaign activation with future slot should succeed");

    println!("✅ Campaign activation succeeded with valid future go_live_slot");
}

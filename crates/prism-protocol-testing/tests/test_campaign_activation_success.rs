use prism_protocol::CampaignStatus;
use prism_protocol_testing::{FixtureStage, TestFixture};

/// Test successful campaign activation
///
/// Verifies that a campaign with all cohorts activated can be successfully
/// activated and transitions from Inactive to Active status, with correct
/// go_live_slot and final_db_ipfs_hash values set.
#[test]
fn test_campaign_activation_success() {
    let mut test = TestFixture::default();

    test.jump_to(FixtureStage::CohortsActivated);

    // Record current slot for later verification
    let activation_slot = test.current_slot();
    let expected_go_live_slot = activation_slot + 10; // try_activate_campaign uses current_slot + 10
    let expected_ipfs_hash = [1u8; 32]; // try_activate_campaign uses all ones

    // Verify initial campaign status is Inactive
    let campaign_before = test
        .fetch_campaign_account()
        .expect("Campaign account should exist");
    assert!(matches!(campaign_before.status, CampaignStatus::Inactive));
    assert_eq!(campaign_before.campaign_db_ipfs_hash, [0u8; 32]); // Should be unset initially
    assert_eq!(campaign_before.go_live_slot, 0); // Should be unset initially

    // Activate the campaign
    test.try_activate_campaign()
        .expect("Campaign activation should succeed");

    // Verify campaign status and parameters are correctly set
    let campaign_after = test
        .fetch_campaign_account()
        .expect("Campaign account should exist");
    assert!(matches!(campaign_after.status, CampaignStatus::Active));
    assert_eq!(campaign_after.campaign_db_ipfs_hash, expected_ipfs_hash);
    assert_eq!(campaign_after.go_live_slot, expected_go_live_slot);
}

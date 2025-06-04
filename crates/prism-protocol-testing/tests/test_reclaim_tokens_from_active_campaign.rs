use prism_protocol_testing::TestFixture;

/// Test reclaim tokens from active campaign → CampaignNotPermanentlyHalted
///
/// Should test:
/// - Set up active campaign with funded vaults
/// - Attempt to reclaim tokens from active campaign
/// - Verify fails with CampaignNotPermanentlyHalted error
/// - Ensure only permanently halted campaigns allow reclamation
#[test]
#[ignore]
fn test_reclaim_tokens_from_active_campaign() {
    let mut _test = TestFixture::default();

    todo!("Implement reclaim from active campaign test - should fail with CampaignNotPermanentlyHalted");
}

use prism_protocol_testing::TestFixture;

/// Test claim from inactive campaign → CampaignNotActive
///
/// Should test:
/// - Set up campaign but DO NOT activate it (leave status = Inactive)
/// - Set up cohort and vault normally
/// - Create valid claimant and merkle proof
/// - Attempt claim_tokens_v0 on inactive campaign
/// - Verify fails with CampaignNotActive error
/// - Also test claiming from paused campaign
/// - Also test claiming from permanently halted campaign
#[test]
#[ignore]
fn test_claim_inactive_campaign() {
    let mut _test = TestFixture::default();

    todo!("Implement inactive campaign claim test - should fail with CampaignNotActive");
}

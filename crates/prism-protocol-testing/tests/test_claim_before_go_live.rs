use prism_protocol_testing::TestFixture;

/// Test claim before go-live slot → GoLiveDateNotReached
///
/// Should test:
/// - Set up campaign with go_live_slot in the future
/// - Activate campaign with future go_live_slot
/// - Create valid claimant and merkle proof
/// - Attempt claim_tokens_v0 before go_live_slot
/// - Verify fails with GoLiveDateNotReached error
/// - Verify no tokens transferred
/// - Test that claim succeeds after warping to go_live_slot
#[test]
#[ignore]
fn test_claim_before_go_live() {
    let mut _test = TestFixture::default();

    todo!("Implement claim before go-live test - should fail with GoLiveDateNotReached");
}

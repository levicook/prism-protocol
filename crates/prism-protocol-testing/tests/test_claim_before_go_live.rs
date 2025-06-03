use prism_protocol_testing::TestFixture;
// use prism_protocol::error::ErrorCode as PrismError;
// use prism_protocol_sdk::build_claim_tokens_v0_ix;
// use solana_instruction::error::InstructionError;
// use solana_transaction_error::TransactionError;

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
#[ignore = "Need to implement custom go-live slot and time-based validation"]
fn test_claim_before_go_live() {
    let mut _test = TestFixture::default();

    todo!("Implement claim before go-live test - should fail with GoLiveDateNotReached");

    // Pseudocode implementation:
    // 1. test.jump_to(FixtureStage::VaultsActivated) // Stop before campaign activation
    // 2. current_slot = test.current_slot()
    // 3. future_go_live_slot = current_slot + 100 // Far in future
    // 4. Manually activate campaign with future go_live_slot
    // 5. Get valid claimant and proof
    // 6. Attempt claim → expect GoLiveDateNotReached error
    // 7. test.advance_slot_by(100) // Warp past go-live
    // 8. Retry same claim → should succeed
}

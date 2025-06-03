use prism_protocol_testing::TestFixture;
// use prism_protocol::error::ErrorCode as PrismError;
// use prism_protocol_sdk::build_claim_tokens_v0_ix;
// use solana_instruction::error::InstructionError;
// use solana_transaction_error::TransactionError;

/// Test duplicate claim prevention via ClaimReceipt PDA
///
/// Should test:
/// - Set up active campaign with funded vault
/// - Create valid claimant and merkle proof
/// - Execute first claim_tokens_v0 successfully
/// - Verify ClaimReceipt PDA is created
/// - Attempt second claim_tokens_v0 with same claimant
/// - Verify fails (ClaimReceipt PDA already exists)
/// - Verify error is account initialization failure (PDA exists)
/// - Verify no additional tokens transferred on second attempt
#[test]
#[ignore = "Need to implement double-claim detection and PDA existence verification"]
fn test_claim_duplicate_prevention() {
    let mut _test = TestFixture::default();

    todo!("Implement duplicate claim prevention test - second claim should fail");

    // Pseudocode implementation:
    // 1. test.jump_to(FixtureStage::CampaignsActivated)
    // 2. test.advance_slot_by(20) // Past go-live
    // 3. Get valid claimant (use early_adopter_1 for consistency)
    // 4. Execute first claim → should succeed
    // 5. Verify ClaimReceipt PDA exists using account_exists()
    // 6. Record vault/claimant balances after first claim
    // 7. Attempt second identical claim → should fail with account creation error
    // 8. Verify balances unchanged from step 6 (no double-spend)
    // 9. Verify ClaimReceipt PDA still exists and unchanged
}

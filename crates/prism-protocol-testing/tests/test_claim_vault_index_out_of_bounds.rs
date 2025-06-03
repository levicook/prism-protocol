use prism_protocol_testing::TestFixture;
// use prism_protocol::error::ErrorCode as PrismError;
// use prism_protocol_sdk::build_claim_tokens_v0_ix;
// use solana_instruction::error::InstructionError;
// use solana_transaction_error::TransactionError;

/// Test claim with vault index out of bounds → AssignedVaultIndexOutOfBounds  
///
/// Should test:
/// - Set up cohort with expected_vault_count = 2
/// - Initialize/activate only vault 0 and vault 1 (indices 0, 1)
/// - Create valid claimant assigned to vault index 2 (out of bounds)
/// - Create valid merkle proof for vault index 2
/// - Attempt claim_tokens_v0 with assigned_vault_index = 2
/// - Verify fails with AssignedVaultIndexOutOfBounds error
/// - Test boundary: vault index = expected_vault_count should fail
#[test]
#[ignore = "Need to implement custom cohort with specific vault count and out-of-bounds assignment"]
fn test_claim_vault_index_out_of_bounds() {
    let mut _test = TestFixture::default();

    todo!(
        "Implement vault index out of bounds test - should fail with AssignedVaultIndexOutOfBounds"
    );

    // Pseudocode implementation:
    // 1. Create custom campaign CSV with claimant assigned to vault index 2
    // 2. Create custom cohort CSV with only 2 vaults (indices 0, 1)
    // 3. Compile campaign → should detect assignment to non-existent vault
    // 4. OR: Use standard campaign but manually build claim instruction with bad vault index
    // 5. test.jump_to(FixtureStage::CampaignsActivated)
    // 6. Get valid claimant but override assigned_vault_index to out-of-bounds value
    // 7. build_claim_tokens_v0_ix with bad vault index
    // 8. Expect AssignedVaultIndexOutOfBounds error
}

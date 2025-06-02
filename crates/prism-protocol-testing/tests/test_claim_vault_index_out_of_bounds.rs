use prism_protocol_testing::TestFixture;

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
#[ignore]
fn test_claim_vault_index_out_of_bounds() {
    let mut _test = TestFixture::default();

    todo!(
        "Implement vault index out of bounds test - should fail with AssignedVaultIndexOutOfBounds"
    );
}

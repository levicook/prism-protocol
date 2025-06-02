use prism_protocol_testing::TestFixture;

/// Test claim with invalid merkle proof → InvalidMerkleProof
///
/// Should test:
/// - Set up active campaign with funded vault
/// - Create claimant with valid entitlements  
/// - Generate INVALID merkle proof (wrong proof, wrong leaf, etc.)
/// - Attempt claim_tokens_v0
/// - Verify fails with InvalidMerkleProof error
/// - Verify no tokens transferred
/// - Verify no ClaimReceipt created
#[test]
#[ignore]
fn test_claim_invalid_merkle_proof() {
    let mut _test = TestFixture::default();

    todo!("Implement invalid merkle proof test - should fail with InvalidMerkleProof");
}

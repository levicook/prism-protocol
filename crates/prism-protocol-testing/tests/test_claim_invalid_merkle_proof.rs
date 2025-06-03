use prism_protocol_testing::TestFixture;
// use prism_protocol::error::ErrorCode as PrismError;
// use prism_protocol_sdk::build_claim_tokens_v0_ix;
// use solana_instruction::error::InstructionError;
// use solana_transaction_error::TransactionError;

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
#[ignore = "Need to implement merkle proof tampering and error verification"]
fn test_claim_invalid_merkle_proof() {
    let mut _test = TestFixture::default();

    todo!("Implement invalid merkle proof test - should fail with InvalidMerkleProof");

    // Pseudocode implementation:
    // 1. test.jump_to(FixtureStage::CampaignsActivated)
    // 2. test.advance_slot_by(20) // Past go-live
    // 3. Get valid claimant from compiled campaign
    // 4. Generate INVALID proof (flip bits, use wrong leaf, etc.)
    // 5. build_claim_tokens_v0_ix with invalid proof
    // 6. Expect transaction failure with InvalidMerkleProof error code
    // 7. Verify vault balance unchanged
    // 8. Verify no ClaimReceipt PDA created
}

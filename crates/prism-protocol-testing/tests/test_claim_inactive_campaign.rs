use prism_protocol_testing::TestFixture;
// use prism_protocol::error::ErrorCode as PrismError;
// use prism_protocol_sdk::build_claim_tokens_v0_ix;
// use solana_instruction::error::InstructionError;
// use solana_transaction_error::TransactionError;

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
#[ignore = "Need to implement campaign status validation and test different inactive states"]
fn test_claim_inactive_campaign() {
    let mut _test = TestFixture::default();

    todo!("Implement inactive campaign claim test - should fail with CampaignNotActive");

    // Pseudocode implementation:
    // 1. test.jump_to(FixtureStage::VaultsActivated) // Stop before campaign activation
    // 2. Verify campaign status is Inactive
    // 3. Get valid claimant and merkle proof  
    // 4. Attempt claim → expect CampaignNotActive error
    // 
    // Additional test cases:
    // 5. Activate campaign, then pause it
    // 6. Attempt claim on paused campaign → expect CampaignNotActive error
    // 7. Permanently halt campaign
    // 8. Attempt claim on halted campaign → expect CampaignNotActive error
}

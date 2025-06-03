/// Test cohort initialization with zero amount_per_entitlement → InvalidEntitlements error
///
/// Should test:
/// - Initialize campaign successfully
/// - Set amount_per_entitlement to 0 (invalid)
/// - Attempt cohort initialization with zero entitlements
/// - Verify fails with InvalidEntitlements error code
/// - Ensure zero-value validation is working properly
#[test]
#[ignore] // TODO: Implement validation for zero amount_per_entitlement
fn test_zero_amount_per_entitlement() {
    todo!(
        "Need to implement API to test zero amount_per_entitlement validation.
        The business rule is valid: amount_per_entitlement should not be zero.
        
        Options to implement:
        1. Add TestFixture API to override campaign data before compilation
        2. Create a separate validation test that directly tests the compiler
        3. Test this scenario through the campaign compiler API with invalid CSV data
        
        The validation should happen during campaign compilation and return an appropriate error."
    );

    /*
    use prism_protocol::error::ErrorCode as PrismError;
    use prism_protocol_testing::{FixtureStage, TestFixture};
    use solana_instruction::error::InstructionError;
    use solana_transaction_error::TransactionError;

    let mut test = TestFixture::default();

    // Get to campaign initialized stage first
    test.jump_to(FixtureStage::CampaignInitialized)
        .expect("campaign initialization failed");

    // Set the next amount per entitlement to zero (should be rejected)
    test.set_next_amount_per_entitlement(0);

    // Try to advance to cohort initialized - should fail with InvalidEntitlements
    let result = test.jump_to(FixtureStage::CohortsInitialized);

    match result {
        Ok(_) => {
            panic!("❌ Cohort initialization should have failed with zero amount per entitlement!");
        }
        Err(failed_meta) => {
            // Verify we got the expected InvalidEntitlements error
            const EXPECTED_ERROR: u32 = PrismError::InvalidEntitlements as u32 + 6000; // Anchor offset

            match failed_meta.err {
                TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
                    assert_eq!(code, EXPECTED_ERROR, "Expected InvalidEntitlements error");
                }
                _ => {
                    panic!(
                        "Expected TransactionError::InstructionError with Custom code, got: {:?}",
                        failed_meta.err
                    );
                }
            }
        }
    }
    */
}

use prism_protocol_testing::TestFixture;

/// Test numeric overflow in claim calculation → NumericOverflow
///
/// Should test:
/// - Set up cohort with amount_per_entitlement = u64::MAX
/// - Create claimant with entitlements = 2 (would overflow u64::MAX * 2)
/// - Create valid merkle proof for this claimant
/// - Attempt claim_tokens_v0
/// - Verify fails with NumericOverflow error
/// - Test edge case: amount_per_entitlement * entitlements = u64::MAX (should succeed)
/// - Test edge case: amount_per_entitlement * entitlements = u64::MAX + 1 (should fail)
#[test]
#[ignore]
fn test_claim_numeric_overflow() {
    let mut _test = TestFixture::default();

    todo!("Implement numeric overflow test - should fail with NumericOverflow");
}

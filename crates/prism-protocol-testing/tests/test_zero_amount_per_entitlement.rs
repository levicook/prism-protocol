use litesvm::LiteSVM;
use prism_protocol_testing::{FixtureStage, FixtureState, TestFixture};

/// Test cohort initialization with zero amount_per_entitlement → InvalidEntitlements error
///
/// Should test:
/// - Initialize campaign successfully
/// - Set amount_per_entitlement to 0 (invalid)
/// - Attempt cohort initialization with zero entitlements
/// - Verify fails with InvalidEntitlements error code
/// - Ensure zero-value validation is working properly
#[ignore = "Needs custom fixture API to override amount_per_entitlement = 0"]
#[tokio::test]
async fn test_zero_amount_per_entitlement() {
    todo!("This test currently doesn't test zero amount_per_entitlement validation - it's a false positive. Need to implement custom fixture support to actually test the zero amount case.");
}

use prism_protocol_testing::TestFixture;

/// Test vault activation with excess funding → IncorrectVaultFunding
///
/// Should test:
/// - Initialize vault and fund with more than required amount
/// - Attempt to activate over-funded vault
/// - Verify fails with IncorrectVaultFunding error
/// - Ensure precise balance validation (exact match required)
#[test]
#[ignore]
fn test_vault_activation_excess_funding() {
    let mut _test = TestFixture::default();

    todo!("Implement vault activation with excess funding test - should fail with IncorrectVaultFunding");
}

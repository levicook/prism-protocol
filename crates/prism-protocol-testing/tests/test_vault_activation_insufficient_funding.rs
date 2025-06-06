use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::CompiledVaultExt;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};
use std::collections::HashMap;

/// Test vault activation with insufficient funding → IncorrectVaultFunding
///
/// Should test:
/// - Initialize vault but fund with less than required amount
/// - Attempt to activate under-funded vault
/// - Verify fails with IncorrectVaultFunding error
/// - Ensure precise balance validation
#[tokio::test]
async fn test_vault_activation_insufficient_funding() {
    let state = FixtureState::simple_v1().await;
    let mut test = TestFixture::new(state, LiteSVM::new())
        .await
        .expect("Failed to create test fixture");

    // Jump to vaults initialized stage
    test.jump_to(FixtureStage::VaultsInitialized).await;

    // Get the single vault address and expected balance
    let vault_info = test.state.ccdb.compiled_vaults().await[0].clone();
    let vault_address = vault_info.vault_address();
    let expected_balance = vault_info.vault_budget_token() - vault_info.vault_dust_token();

    // Fund vault with INSUFFICIENT amount (expected - 1 token)
    let insufficient_amount = expected_balance.saturating_sub(1);
    let custom_funding = HashMap::from([(vault_address, insufficient_amount)]);

    println!(
        "💰 Funding vault with {} tokens (expected: {})",
        insufficient_amount, expected_balance
    );

    test.try_fund_vaults_with_custom_amounts(custom_funding)
        .await
        .expect("Custom vault funding should succeed");

    // Try to activate vault - should fail because it has insufficient balance
    let result = test.try_activate_vaults().await;

    demand_prism_error(
        result,
        PrismError::IncorrectVaultFunding as u32,
        "IncorrectVaultFunding",
    );

    println!("✅ Correctly prevented vault activation with insufficient funding");
}

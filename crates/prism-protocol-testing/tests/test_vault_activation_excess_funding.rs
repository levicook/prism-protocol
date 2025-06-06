use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::CompiledVaultExt;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};

/// Test vault activation with excess funding → IncorrectVaultFunding
///
/// Should test:
/// - Initialize vault and fund with more than required amount
/// - Attempt to activate over-funded vault
/// - Verify fails with IncorrectVaultFunding error
/// - Ensure precise balance validation (exact match required)
#[tokio::test]
async fn test_vault_activation_excess_funding() {
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

    // First fund normally to get the correct expected balance
    test.try_fund_vaults()
        .await
        .expect("Normal vault funding should succeed");

    // Then add just 1 excess token to make it over-funded
    let admin_token_account = test.state.address_finder().find_admin_token_account();

    // Mint 1 extra token to admin for the excess
    test.mint_to(&admin_token_account, 1)
        .expect("Minting extra token should succeed");

    // Transfer the excess token to vault
    let transfer_ix = spl_token::instruction::transfer(
        &test.state.address_finder().token_program_id,
        &admin_token_account,
        &vault_address,
        &test.state.admin_address(),
        &[&test.state.admin_address()],
        1, // Transfer the 1 excess token
    )
    .expect("Failed to build transfer ix");

    test.send_instructions(&[transfer_ix])
        .expect("Transferring excess token should succeed");

    println!(
        "💰 Added 1 excess token to vault (now has {} + 1)",
        expected_balance
    );

    // Try to activate vault - should fail because it has excess balance
    let result = test.try_activate_vaults().await;

    demand_prism_error(
        result,
        PrismError::IncorrectVaultFunding as u32,
        "IncorrectVaultFunding",
    );

    println!("✅ Correctly prevented vault activation with excess funding");
}

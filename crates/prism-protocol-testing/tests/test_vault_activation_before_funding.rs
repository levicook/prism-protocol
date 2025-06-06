use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};

/// Test vault activation before funding (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign, cohorts, and vaults but skip funding
/// - Attempt to activate vaults WITHOUT funding them first
/// - Verify operation fails (vault activation requires sufficient balance)
/// - Ensure proper order dependencies are enforced
#[tokio::test]
async fn test_vault_activation_before_funding() {
    let state = FixtureState::rand().await;
    let mut test = TestFixture::new(state, LiteSVM::new())
        .await
        .expect("Failed to create test fixture");

    test.jump_to(FixtureStage::VaultsInitialized).await;

    let result = test.try_activate_vaults().await;

    // Vault activation without funding should fail with incorrect vault funding
    demand_prism_error(
        result,
        PrismError::IncorrectVaultFunding as u32,
        "IncorrectVaultFunding",
    );

    println!("✅ Correctly prevented vault activation before funding");
}

use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, TestFixture};

/// Test vault activation before funding (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign, cohorts, and vaults but skip funding
/// - Attempt to activate vaults WITHOUT funding them first
/// - Verify operation fails (vault activation requires sufficient balance)
/// - Ensure proper order dependencies are enforced
#[test]
fn test_vault_activation_before_funding() {
    let mut test = TestFixture::default();

    test.jump_to(FixtureStage::VaultsInitialized);

    let result = test.try_activate_vaults();

    // Vault activation without funding should fail with incorrect vault funding
    demand_prism_error(
        result,
        PrismError::IncorrectVaultFunding as u32,
        "IncorrectVaultFunding",
    );

    println!("✅ Correctly prevented vault activation before funding");
}

use prism_protocol_testing::{demand_invalid_account_data_error, FixtureStage, TestFixture};

/// Test vault funding before initialization (wrong order) - should fail
///
/// Should test:
/// - Initialize campaign and cohorts only
/// - Attempt to fund vaults WITHOUT vaults being initialized first
/// - Verify operation fails (vault funding requires vault to exist first)
/// - Ensure proper order dependencies are enforced
#[test]
fn test_vault_funding_before_initialization() {
    let mut test = TestFixture::default();

    test.jump_to(FixtureStage::CohortsInitialized);

    let result = test.try_fund_vaults();

    demand_invalid_account_data_error(result);

    println!("✅ Correctly prevented vault funding before initialization");
}

use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, TestFixture};

/// Test cohort activation before vault activation (wrong order) - should fail
///
/// Should test:
/// - Initialize and fund vaults but do not activate them
/// - Attempt to activate cohorts WITHOUT all vaults being activated first
/// - Verify operation fails with NotAllVaultsActivated error
/// - Ensure proper order dependencies are enforced
#[ignore]
#[test]
fn test_cohort_activation_before_vault_activation() {
    let mut test = TestFixture::default();

    test.jump_to(FixtureStage::VaultsFunded);

    let result = test.try_activate_cohorts();

    demand_prism_error(
        result,
        PrismError::NotAllVaultsActivated as u32,
        "NotAllVaultsActivated",
    );

    println!("✅ Correctly prevented cohort activation before vault activation");
}

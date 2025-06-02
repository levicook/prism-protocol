use prism_protocol_testing::{FixtureStage, TestFixture};

#[test]
#[ignore]
fn test_vault_initialization_before_cohort() {
    let mut test = TestFixture::default();

    // Setup campaign but not cohort
    test.jump_to(FixtureStage::CampaignInitialized)
        .expect("campaign initialization failed");

    // Try to initialize vault without cohort
    let result = test.jump_to(FixtureStage::VaultInitialized);

    // This should fail because no cohort exists yet
    assert!(
        result.is_err(),
        "Expected vault initialization to fail without cohort"
    );
    println!("✅ Correctly prevented premature vault initialization");
}

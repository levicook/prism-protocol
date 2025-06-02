use prism_protocol_testing::{FixtureStage, TestFixture};

#[test]
#[ignore]
fn test_vault_activation_before_initialization() {
    let mut test = TestFixture::default();

    // Setup campaign and cohort but not vault
    test.jump_to(FixtureStage::CampaignInitialized)
        .expect("campaign initialization failed");

    test.jump_to(FixtureStage::CohortInitialized)
        .expect("cohort initialization failed");

    // Try to activate vault without initializing it first
    let result = test.jump_to(FixtureStage::VaultActivated);

    // This should fail because vault isn't initialized yet
    assert!(
        result.is_err(),
        "Expected vault activation to fail without vault initialization"
    );
    println!("✅ Correctly prevented premature vault activation");
}

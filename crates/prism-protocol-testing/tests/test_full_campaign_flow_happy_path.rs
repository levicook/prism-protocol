use prism_protocol_testing::{FixtureStage, TestFixture};

/// Test the full campaign initialization and activation flow (happy path)
///
/// Should test:
/// - Initialize campaign → cohort → vault in correct order
/// - Fund vault with correct amount
/// - Activate vault → cohort → campaign successfully
/// - Verify all status transitions work correctly
/// - **NOTE: Does NOT test claiming - that's the critical missing piece**
#[test]
fn test_full_campaign_flow_happy_path() {
    let mut test = TestFixture::default();

    // Step 1: Initialize Campaign
    test.jump_to(FixtureStage::CampaignInitialized)
        .expect("campaign initialization failed");

    // Step 2: Initialize Cohort
    test.jump_to(FixtureStage::CohortInitialized)
        .expect("cohort initialization failed");

    // Step 3: Initialize Vault
    test.jump_to(FixtureStage::VaultInitialized)
        .expect("vault initialization failed");

    // Step 4: Activate Vault (fund it)
    test.jump_to(FixtureStage::VaultActivated)
        .expect("vault activation failed");

    // Step 5: Activate Cohort
    test.jump_to(FixtureStage::CohortActivated)
        .expect("cohort activation failed");

    // Step 6: Activate Campaign (go live)
    test.jump_to(FixtureStage::CampaignActivated)
        .expect("campaign activation failed");

    // TODO: Add claiming flow once TestFixture supports it
    println!("✅ Full campaign flow completed successfully!");
}

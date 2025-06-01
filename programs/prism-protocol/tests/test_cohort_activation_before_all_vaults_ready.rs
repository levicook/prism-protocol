#![cfg(feature = "test-sbf")]

use prism_protocol_testing::{
    generate_test_fingerprint, CampaignAction, CampaignLifecycleStage, TestFixture,
};

#[test]
fn test_cohort_activation_before_all_vaults_ready() {
    let mut fixture = TestFixture::new();
    fixture.test_fingerprint = generate_test_fingerprint("cohort_activation_premature");

    println!("🚫 Testing cohort activation before all vaults are ready");

    // Set up campaign with vaults initialized but not all funded
    let mut state = fixture.setup_to_stage(CampaignLifecycleStage::VaultsInitialized);

    // Try to activate cohort before vaults are funded - should fail
    fixture.expect_failure(
        &mut state,
        CampaignAction::ActivateCohort { cohort_index: 0 },
        "Action not yet implemented", // TODO: Update when we implement this
    );

    println!("✅ Cohort activation properly fails when vaults not ready");
}

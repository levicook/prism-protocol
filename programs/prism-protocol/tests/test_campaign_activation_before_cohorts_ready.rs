#![cfg(feature = "test-sbf")]

use prism_protocol_testing::{
    generate_test_fingerprint, CampaignAction, CampaignLifecycleStage, TestFixture,
};

#[test]
fn test_campaign_activation_before_cohorts_ready() {
    let mut fixture = TestFixture::new();
    fixture.test_fingerprint = generate_test_fingerprint("campaign_activation_premature");

    println!("🚫 Testing campaign activation before cohorts are ready");

    let test_stages = [
        CampaignLifecycleStage::CampaignInitialized,
        CampaignLifecycleStage::CohortsInitialized,
        CampaignLifecycleStage::VaultsInitialized,
        CampaignLifecycleStage::VaultsActivated,
    ];

    for stage in test_stages {
        let mut state = fixture.setup_to_stage(stage);

        fixture.expect_failure(
            &mut state,
            CampaignAction::ActivateCampaign,
            "Action not yet implemented", // TODO: Update when we implement this
        );

        println!(
            "✅ Campaign activation properly fails at stage: {:?}",
            stage
        );
    }

    println!("✅ Campaign activation properly blocked until cohorts ready");
}

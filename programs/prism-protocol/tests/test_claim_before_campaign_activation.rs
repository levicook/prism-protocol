#![cfg(feature = "test-sbf")]

use prism_protocol_testing::{
    generate_test_fingerprint, CampaignAction, CampaignLifecycleStage, TestClaimants, TestFixture,
};

#[test]
fn test_claim_before_campaign_activation() {
    let mut fixture = TestFixture::new();
    fixture.test_fingerprint = generate_test_fingerprint("claim_before_activation");
    let claimants = TestClaimants::new();

    println!("🚫 Testing claims before campaign activation");

    // Test claims fail at each stage before activation
    let test_stages = [
        CampaignLifecycleStage::CampaignInitialized,
        CampaignLifecycleStage::CohortsInitialized,
        CampaignLifecycleStage::VaultsInitialized,
        CampaignLifecycleStage::VaultsActivated,
        CampaignLifecycleStage::CohortsActivated,
    ];

    for stage in test_stages {
        let mut state = fixture.setup_to_stage(stage);

        fixture.expect_failure(
            &mut state,
            CampaignAction::ClaimTokens {
                claimant: claimants.alice,
            },
            "Campaign not activated",
        );

        println!("✅ Claims properly fail at stage: {:?}", stage);
    }

    // But claims should work once campaign is activated
    let mut state = fixture.setup_to_stage(CampaignLifecycleStage::CampaignActivated);
    fixture.expect_success(
        &mut state,
        CampaignAction::ClaimTokens {
            claimant: claimants.alice,
        },
    );

    println!("✅ Claims work correctly after campaign activation");
}

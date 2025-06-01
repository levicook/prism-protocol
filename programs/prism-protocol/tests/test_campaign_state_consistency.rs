#![cfg(feature = "test-sbf")]

use prism_protocol_testing::{generate_test_fingerprint, CampaignLifecycleStage, TestFixture};

#[test]
fn test_campaign_state_consistency() {
    let mut fixture = TestFixture::new();
    fixture.test_fingerprint = generate_test_fingerprint("campaign_state_consistency");

    println!("🔍 Testing campaign state consistency across all stages");

    // Test that advancing through stages maintains consistency
    let mut state = fixture.setup_to_stage(CampaignLifecycleStage::CampaignInitialized);
    let original_campaign_address = state.campaign.address;
    let original_mint = state.campaign.mint;

    // Advance through all stages and verify consistency
    let stages = [
        CampaignLifecycleStage::CohortsInitialized,
        CampaignLifecycleStage::VaultsInitialized,
        CampaignLifecycleStage::VaultsActivated,
        CampaignLifecycleStage::CampaignActivated,
    ];

    for stage in stages {
        fixture.advance_to_stage(&mut state, stage);

        // Core identifiers should remain consistent
        assert_eq!(state.campaign.address, original_campaign_address);
        assert_eq!(state.campaign.mint, original_mint);

        // Stage should match what we requested
        assert_eq!(state.stage, stage);

        println!("✅ State consistency verified for stage: {:?}", stage);
    }

    println!("✅ Campaign state consistency maintained across all stages");
}

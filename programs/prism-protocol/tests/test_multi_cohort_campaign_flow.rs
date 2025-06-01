#![cfg(feature = "test-sbf")]

use prism_protocol_testing::{generate_test_fingerprint, CampaignLifecycleStage, TestFixture};

#[test]
fn test_multi_cohort_campaign_flow() {
    let mut fixture = TestFixture::new();
    fixture.test_fingerprint = generate_test_fingerprint("multi_cohort_campaign");

    println!("🚀 Testing multi-cohort campaign with different configurations");

    // Test with more realistic cohort configurations
    let state = fixture.setup_to_stage(CampaignLifecycleStage::VaultsActivated);

    // Verify cohorts have different configurations
    assert_eq!(state.cohorts.len(), 2);
    assert_eq!(state.vaults[0].len(), 2); // Small cohort
    assert_eq!(state.vaults[1].len(), 3); // Larger cohort

    // All vaults should be funded and ready
    for (cohort_idx, cohort_vaults) in state.funded_vaults.iter().enumerate() {
        for (vault_idx, &is_funded) in cohort_vaults.iter().enumerate() {
            assert!(
                is_funded,
                "Cohort {} vault {} should be funded",
                cohort_idx, vault_idx
            );
        }
    }

    println!("✅ Multi-cohort campaign flow working correctly");
}

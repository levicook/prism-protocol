#![cfg(feature = "test-sbf")]

use {
    anchor_lang::prelude::AccountDeserialize,
    prism_protocol::state::CampaignV0,
    prism_protocol_testing::{
        generate_test_fingerprint, CampaignAction, CampaignLifecycleStage, TestClaimants,
        TestFixture,
    },
};

#[test]
fn test_complete_campaign_happy_path() {
    let mut fixture = TestFixture::new();
    fixture.test_fingerprint = generate_test_fingerprint("complete_campaign_happy_path");

    println!("🚀 Starting complete campaign happy path test");

    // Stage 1: Campaign Initialization
    let mut state = fixture.setup_to_stage(CampaignLifecycleStage::CampaignInitialized);
    assert_eq!(state.stage, CampaignLifecycleStage::CampaignInitialized);
    println!("✅ Stage 1: Campaign initialized successfully");

    // Stage 2: Cohorts Initialization
    fixture.advance_to_stage(&mut state, CampaignLifecycleStage::CohortsInitialized);
    assert_eq!(state.stage, CampaignLifecycleStage::CohortsInitialized);
    assert_eq!(state.cohorts.len(), 2);
    println!("✅ Stage 2: Cohorts initialized successfully");

    // Stage 3: Vaults Initialization
    fixture.advance_to_stage(&mut state, CampaignLifecycleStage::VaultsInitialized);
    assert_eq!(state.stage, CampaignLifecycleStage::VaultsInitialized);
    assert_eq!(state.vaults[0].len(), 2); // First cohort: 2 vaults
    assert_eq!(state.vaults[1].len(), 3); // Second cohort: 3 vaults
    println!("✅ Stage 3: Vaults initialized successfully");

    // Stage 4: Vaults Funding (Activation)
    fixture.advance_to_stage(&mut state, CampaignLifecycleStage::VaultsActivated);
    assert_eq!(state.stage, CampaignLifecycleStage::VaultsActivated);
    // Verify all vaults are funded
    for cohort_vaults in &state.funded_vaults {
        for &is_funded in cohort_vaults {
            assert!(is_funded, "All vaults should be funded at this stage");
        }
    }
    println!("✅ Stage 4: Vaults funded successfully");

    // Stage 5: Campaign Activation (TODO: implement when we have the instruction)
    fixture.advance_to_stage(&mut state, CampaignLifecycleStage::CampaignActivated);
    assert_eq!(state.stage, CampaignLifecycleStage::CampaignActivated);
    println!("✅ Stage 5: Campaign activated successfully");

    // Stage 6: Claims should now work
    let claimants = TestClaimants::new();
    fixture.expect_success(
        &mut state,
        CampaignAction::ClaimTokens {
            claimant: claimants.alice,
        },
    );
    fixture.expect_success(
        &mut state,
        CampaignAction::ClaimTokens {
            claimant: claimants.bob,
        },
    );
    println!("✅ Stage 6: Token claims working successfully");

    // Verify final campaign state
    let campaign_account = &state.campaign.campaign_account;
    let campaign_state = CampaignV0::try_deserialize(&mut campaign_account.data.as_slice())
        .expect("Failed to deserialize final campaign state");

    // Should have processed cohorts and be active
    assert_eq!(campaign_state.expected_cohort_count, 2);
    assert_eq!(campaign_state.initialized_cohort_count, 2);

    println!("🎉 Complete campaign happy path test PASSED!");
    println!("   Campaign: {}", state.campaign.address);
    println!("   Cohorts: {}", state.cohorts.len());
    println!(
        "   Total Vaults: {}",
        state.vaults.iter().map(|v| v.len()).sum::<usize>()
    );
}

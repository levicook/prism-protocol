use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{
    demand_prism_error, deterministic_keypair, CampaignSnapshot, FixtureStage, TestFixture,
};
use solana_signer::Signer as _;

/// Test claim from paused campaign → CampaignNotActive
///
/// This test validates that claims are properly blocked when campaigns are paused:
/// - Verifies campaign lifecycle validation prevents claims on paused campaigns
/// - Ensures proper error handling for claims during temporary suspension
/// - Confirms no state changes occur during blocked claims
///
/// **Scenario**: Campaign was active, then admin paused it (status = Paused)
#[test]
fn test_claim_paused_campaign() {
    println!("🧪 Testing claim from paused campaign...");

    let mut test = TestFixture::default();

    // 1. Fully activate campaign first
    test.jump_to(FixtureStage::CampaignActivated);
    test.advance_slot_by(20); // Past go-live

    // 2. Pause the campaign
    test.try_pause_campaign()
        .expect("Should be able to pause active campaign");

    // 3. Get claimant and capture state
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();
    test.airdrop(&claimant_pubkey, 1_000_000_000);

    let state_before = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);

    // 4. Attempt claim on paused campaign (should fail)
    let result = test.try_claim_tokens(&claimant_keypair);

    // 5. Verify fails with CampaignNotActive error
    demand_prism_error(
        result,
        PrismError::CampaignNotActive as u32,
        "CampaignNotActive",
    );

    // 6. Verify no state changes occurred
    let state_after = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);
    assert_eq!(
        state_before, state_after,
        "No state should change during blocked claim"
    );

    println!("✅ Paused campaign correctly blocked claim with CampaignNotActive");
}

use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{
    demand_prism_error, deterministic_keypair, CampaignSnapshot, FixtureStage, TestFixture,
};
use solana_signer::Signer as _;

/// Test claim from inactive campaign → CampaignNotActive
///
/// This test validates that claims are properly blocked when campaigns are inactive:
/// - Verifies campaign lifecycle validation prevents claims on never-activated campaigns
/// - Ensures proper error handling for premature claim attempts
/// - Confirms no state changes occur during blocked claims
///
/// **Scenario**: Campaign deployed with vaults funded but never activated (status = Inactive)
#[test]
fn test_claim_inactive_campaign() {
    println!("🧪 Testing claim from inactive campaign...");

    let mut test = TestFixture::default();

    // 1. Set up campaign but STOP before activation (status = Inactive)
    test.jump_to(FixtureStage::VaultsActivated);

    // 2. Get claimant and capture state before attempt
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();
    test.airdrop(&claimant_pubkey, 1_000_000_000);

    let state_before = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);

    // 3. Attempt claim using TestFixture helper (should fail)
    let result = test.try_claim_tokens(&claimant_keypair);

    // 4. Verify fails with CampaignNotActive error
    demand_prism_error(
        result,
        PrismError::CampaignNotActive as u32,
        "CampaignNotActive",
    );

    // 5. Verify no state changes occurred
    let state_after = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);
    assert_eq!(
        state_before, state_after,
        "No state should change during blocked claim"
    );

    println!("✅ Inactive campaign correctly blocked claim with CampaignNotActive");
}

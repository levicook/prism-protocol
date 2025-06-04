use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::{
    build_claim_tokens_v0_ix, compile_campaign, AddressFinder, CampaignCsvRow, CohortsCsvRow,
};
use prism_protocol_testing::{
    demand_prism_error, deterministic_keypair, CampaignSnapshot, FixtureStage, FixtureState,
    TestFixture,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;

/// Test numeric overflow in claim calculation → NumericOverflow
///
/// This test validates the critical security boundary that prevents arithmetic overflow
/// in token claim calculations:
/// - Tests checked_mul protection in amount_per_entitlement * entitlements
/// - Verifies proper error handling for extreme values that would cause u64 overflow
/// - Ensures no state changes occur when overflow is detected
///
/// **Security Importance**: Without this protection, attackers could potentially:
/// - Claim more tokens than intended by causing integer overflow
/// - Exploit wrapping arithmetic to claim massive amounts
/// - Bypass intended distribution limits
///
/// **Test Strategy**: Create custom campaign with extreme amount_per_entitlement
/// values that cause overflow when multiplied by legitimate entitlements.
#[test]
fn test_claim_numeric_overflow() {
    println!("🧪 Testing numeric overflow protection in claim calculation...");

    // 1. Create custom campaign with extreme amount_per_entitlement
    let mut test = create_extreme_value_test_fixture();

    // 2. Set up campaign normally through to activation
    test.jump_to(FixtureStage::CampaignActivated);
    test.advance_slot_by(20); // Past go-live

    // 3. Get legitimate claimant and extract campaign data
    let claimant_keypair = deterministic_keypair("extreme_claimant");
    let claimant_pubkey = claimant_keypair.pubkey();
    test.airdrop(&claimant_pubkey, 1_000_000_000);

    let (cohort, leaf) = test
        .state
        .compiled_campaign
        .find_claimant_in_cohort(&claimant_pubkey, "ExtremeValueCohort")
        .expect("extreme_claimant should be in ExtremeValueCohort");

    let claimant_token_account =
        get_associated_token_address(&claimant_pubkey, &test.state.compiled_campaign.mint);

    // 4. Generate legitimate merkle proof for the actual leaf values
    let proof = cohort
        .proof_for_claimant(&claimant_pubkey)
        .expect("Should be able to generate proof");

    // 5. Display the extreme values that will cause overflow
    let amount_per_entitlement = cohort.amount_per_entitlement.floor().to_u64().unwrap();
    let legitimate_entitlements = leaf.entitlements;

    println!("📊 Extreme value overflow scenario:");
    println!(
        "  Amount per entitlement: {} (extreme value)",
        amount_per_entitlement
    );
    println!("  Legitimate entitlements: {}", legitimate_entitlements);
    println!(
        "  Calculation: {} * {} > u64::MAX",
        amount_per_entitlement, legitimate_entitlements
    );

    // Verify our setup will actually cause overflow
    assert!(
        amount_per_entitlement
            .checked_mul(legitimate_entitlements)
            .is_none(),
        "Test setup error: should cause overflow"
    );

    // 6. Capture state before overflow attempt
    let state_before = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);

    // 7. Build claim instruction with legitimate values that cause overflow due to extreme amount_per_entitlement
    let (overflow_claim_ix, _, _) = build_claim_tokens_v0_ix(
        &test.state.address_finder,
        test.state.compiled_campaign.admin,
        claimant_pubkey,
        test.state.compiled_campaign.mint,
        claimant_token_account,
        test.state.compiled_campaign.fingerprint,
        cohort.merkle_root,
        proof,
        leaf.assigned_vault_index,
        legitimate_entitlements, // ← This is legitimate, but amount_per_entitlement is extreme
    )
    .expect("Failed to build claim tokens v0 ix");

    // 8. Attempt claim with overflow-inducing calculation
    let overflow_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[overflow_claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let result = test.send_transaction(overflow_tx);

    // 9. Verify fails with NumericOverflow error
    demand_prism_error(
        result,
        PrismError::NumericOverflow as u32,
        "NumericOverflow",
    );

    println!("✅ Numeric overflow correctly detected and blocked");

    // 10. Verify no state changes occurred
    let state_after = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);
    assert_eq!(
        state_before, state_after,
        "No state should change when overflow is detected"
    );

    println!("✅ Verified no state changes during blocked overflow attempt");

    println!("🎉 Numeric overflow protection test completed successfully!");
}

/// Create a TestFixture with extreme amount_per_entitlement values that cause overflow
fn create_extreme_value_test_fixture() -> TestFixture {
    let address_finder = AddressFinder::default();
    let admin_keypair = deterministic_keypair("extreme_admin");
    let mint_keypair = deterministic_keypair("extreme_mint");

    // Create campaign with claimant that will definitely cause overflow
    let campaign_rows = vec![CampaignCsvRow {
        cohort: "ExtremeValueCohort".to_string(),
        claimant: deterministic_keypair("extreme_claimant").pubkey(),
        entitlements: 3, // 3 entitlements - will overflow when multiplied by amount_per_entitlement
    }];

    // Calculate a budget that forces amount_per_entitlement to be > u64::MAX / 3
    // This guarantees overflow when multiplied by 3 entitlements
    let base_amount = u64::MAX / 3 + 1000; // Slightly more than safe threshold
    let extreme_budget = Decimal::from(base_amount) * Decimal::from(3); // Total budget

    let cohorts_rows = vec![CohortsCsvRow {
        cohort: "ExtremeValueCohort".to_string(),
        share_percentage: Decimal::from(100), // 100% of budget
    }];

    let compiled_campaign = compile_campaign(
        address_finder.clone(),
        &campaign_rows,
        &cohorts_rows,
        extreme_budget,
        mint_keypair.pubkey(),
        0, // 0 decimals to avoid decimal math complications
        admin_keypair.pubkey(),
        1, // 1 claimant per vault
    )
    .expect("Failed to compile extreme value campaign");

    // Verify our extreme setup worked
    let cohort = &compiled_campaign.cohorts[0];
    let amount_per_entitlement = cohort.amount_per_entitlement.floor().to_u64().unwrap();
    let total_entitlements = 3u64; // From our claimant

    println!("🚀 Created extreme value campaign:");
    println!("  Amount per entitlement: {}", amount_per_entitlement);
    println!("  Total entitlements: {}", total_entitlements);
    println!(
        "  Will overflow: {}",
        amount_per_entitlement
            .checked_mul(total_entitlements)
            .is_none()
    );

    // Ensure this will actually cause overflow
    assert!(
        amount_per_entitlement
            .checked_mul(total_entitlements)
            .is_none(),
        "Extreme setup failed - should cause overflow"
    );

    let state = FixtureState {
        address_finder,
        admin_keypair,
        mint_keypair,
        compiled_campaign,
        stage: FixtureStage::default(),
    };

    TestFixture::new(state, LiteSVM::new()).expect("Failed to create extreme value test fixture")
}

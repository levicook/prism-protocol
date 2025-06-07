use litesvm::LiteSVM;
use prism_protocol_sdk::CompiledCohortExt;
use prism_protocol_testing::{
    deterministic_keypair, CampaignSnapshot, FixtureStage, FixtureState, TestFixture,
};
use solana_signer::Signer;

/// Test duplicate claim prevention via ClaimReceipt PDA
///
/// This test validates that claimants cannot claim tokens multiple times by:
/// - Using ClaimReceipt PDAs as duplicate prevention mechanism
/// - Verifying proper error handling when attempting duplicate claims
/// - Ensuring no state changes during blocked duplicate attempts
///
/// **Test flow:**
/// 1. Set up active campaign (past go-live)
/// 2. Execute first successful claim using TestFixture helper
/// 3. Verify ClaimReceipt PDA creation
/// 4. Manually attempt duplicate claim with same parameters
/// 5. Verify duplicate fails and no state changes occur
#[tokio::test]
async fn test_claim_duplicate_prevention() {
    let mut test = TestFixture::new(FixtureState::rand().await, LiteSVM::new())
        .await
        .unwrap();

    // 1. Set up active campaign (past go-live)
    test.jump_to(FixtureStage::CampaignActivated).await;
    test.advance_slot_by(20); // Past go-live

    // 2. Get claimant (use early_adopter_1 for consistency with other tests)
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    // 3. Execute first claim using TestFixture helper → should succeed
    test.airdrop(&claimant_pubkey, 1_000_000_000);
    test.try_claim_tokens(&claimant_keypair)
        .await
        .expect("First claim should succeed");

    println!("✅ First claim succeeded using TestFixture helper");

    // 4. Verify ClaimReceipt PDA was created (find early_adopter_1's cohort)
    let cohorts = test.state.compiled_cohorts().await;
    let early_adopters_cohort = cohorts
        .iter()
        .find(|c| c.cohort_csv_row_id == 1) // Assuming EarlyAdopters is first
        .expect("EarlyAdopters cohort should exist");

    let cohort_address = early_adopters_cohort.address();

    let (claim_receipt_address, _) = test
        .state
        .address_finder()
        .find_claim_receipt_v0_address(&cohort_address, &claimant_pubkey);

    assert!(
        test.account_exists(&claim_receipt_address),
        "ClaimReceipt PDA should be created after first claim"
    );

    println!("✅ ClaimReceipt PDA created successfully");

    // 5. Capture state after first claim
    let state_after_first_claim =
        CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]).await;

    println!("📊 State after first claim:");
    println!(
        "  Claimant balance: {}",
        state_after_first_claim
            .tracked_claimants
            .get(&claimant_pubkey)
            .copied()
            .unwrap_or(0)
    );
    println!(
        "  Total vault balance: {}",
        state_after_first_claim.total_vault_balance()
    );

    // 6. Attempt duplicate claim using the same high-level API (this is what we're testing)
    println!("🔄 Attempting duplicate claim (should fail)...");

    let duplicate_result = test.try_claim_tokens(&claimant_keypair).await;

    // 7. Verify duplicate claim fails appropriately
    match duplicate_result {
        Ok(_) => {
            panic!("Duplicate claim should have failed but succeeded!");
        }
        Err(failed_meta) => {
            println!("✅ Duplicate claim correctly failed: {:?}", failed_meta.err);
            // Note: The specific error depends on implementation:
            // - Custom PrismError for already claimed
            // - Solana's AccountAlreadyExists error
            // - Anchor's ConstraintRaw or similar
            // The key is that it fails deterministically
        }
    }

    // 8. Verify no state changes during duplicate attempt
    let state_after_duplicate_attempt =
        CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]).await;

    assert_eq!(
        state_after_first_claim, state_after_duplicate_attempt,
        "No state should change when duplicate claim is blocked"
    );

    println!("✅ Verified no state changes during blocked duplicate claim");

    // 9. Verify ClaimReceipt PDA integrity maintained
    assert!(
        test.account_exists(&claim_receipt_address),
        "ClaimReceipt PDA should still exist after duplicate attempt"
    );

    println!("✅ ClaimReceipt PDA integrity maintained");

    println!("🎉 Duplicate claim prevention test completed successfully!");
    println!("   ✅ First claim succeeded with TestFixture helper");
    println!("   ❌ Duplicate claim correctly failed");
    println!("   🔬 Verified no state changes during duplicate attempt");
    println!("   📋 ClaimReceipt PDA integrity maintained");

    // 🎓 KEY LEARNINGS FOR DEVELOPERS:
    //
    // 1. PDA-BASED DUPLICATE PREVENTION: Using Program Derived Addresses (PDAs)
    //    as "receipts" is a robust pattern for preventing duplicate operations.
    //    Once created, a PDA with the same seeds cannot be created again.
    //
    // 2. TESTING PATTERNS: Use high-level helpers (try_claim_tokens) for the
    //    happy path, then manually construct edge cases to test specific behaviors.
    //
    // 3. STATE ISOLATION: Failed transactions should not modify program state.
    //    The CampaignSnapshot pattern enables surgical verification of this property.
    //
    // 4. IDEMPOTENCY: Well-designed blockchain operations should be idempotent -
    //    repeated calls should not cause additional state changes beyond the first.
}

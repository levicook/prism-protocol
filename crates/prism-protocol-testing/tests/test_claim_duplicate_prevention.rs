use prism_protocol_sdk::build_claim_tokens_v0_ix;
use prism_protocol_testing::{deterministic_keypair, CampaignSnapshot, FixtureStage, TestFixture};
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;

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
#[test]
fn test_claim_duplicate_prevention() {
    let mut test = TestFixture::default();

    // 1. Set up active campaign (past go-live)
    test.jump_to(FixtureStage::CampaignActivated);
    test.advance_slot_by(20); // Past go-live

    // 2. Get claimant (use early_adopter_1 for consistency with other tests)
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    // 3. Execute first claim using TestFixture helper → should succeed
    test.airdrop(&claimant_pubkey, 1_000_000_000);
    test.try_claim_tokens(&claimant_keypair)
        .expect("First claim should succeed");

    println!("✅ First claim succeeded using TestFixture helper");

    // 4. Verify ClaimReceipt PDA was created (extract minimal data needed)
    let (cohort_address, _) = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");
        (cohort.address, ())
    };

    let (claim_receipt_address, _) = test
        .state
        .address_finder
        .find_claim_receipt_v0_address(&cohort_address, &claimant_pubkey);

    assert!(
        test.account_exists(&claim_receipt_address),
        "ClaimReceipt PDA should be created after first claim"
    );

    println!("✅ ClaimReceipt PDA created successfully");

    // 5. Capture state after first claim
    let state_after_first_claim =
        CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);

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

    // 6. Manually attempt duplicate claim (this is what we're testing)
    println!("🔄 Attempting duplicate claim (should fail)...");

    // Extract only the data needed for manual claim attempt
    let (admin, mint, fingerprint, merkle_root, assigned_vault_index, entitlements) = {
        let (cohort, leaf) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");

        (
            test.state.compiled_campaign.admin,
            test.state.compiled_campaign.mint,
            test.state.compiled_campaign.fingerprint,
            cohort.merkle_root,
            leaf.assigned_vault_index,
            leaf.entitlements,
        )
    };

    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    // Generate proof for duplicate attempt
    let proof = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");
        cohort
            .proof_for_claimant(&claimant_pubkey)
            .expect("Should be able to generate proof")
    };

    // Build duplicate claim instruction with identical parameters
    let (duplicate_claim_ix, _, _) = build_claim_tokens_v0_ix(
        &test.state.address_finder,
        admin,
        claimant_pubkey,
        mint,
        claimant_token_account,
        fingerprint,
        merkle_root,
        proof,
        assigned_vault_index,
        entitlements,
    )
    .expect("Failed to build duplicate claim instruction");

    let duplicate_claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[duplicate_claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let duplicate_result = test.send_transaction(duplicate_claim_tx);

    // 7. Verify duplicate claim fails appropriately
    match duplicate_result {
        Ok(_) => {
            panic!("Duplicate claim should have failed but succeeded!");
        }
        Err(failed_meta) => {
            println!("✅ Duplicate claim correctly failed: {:?}", failed_meta.err);
            // Note: The specific error code depends on implementation:
            // - Custom PrismError for already claimed
            // - Solana's AccountAlreadyInitialized error
            // - Anchor's ConstraintRaw or similar
            // The key is that it fails deterministically
        }
    }

    // 8. Verify no state changes during duplicate attempt
    let state_after_duplicate_attempt =
        CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);

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

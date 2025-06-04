use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::build_claim_tokens_v0_ix;
use prism_protocol_testing::{
    demand_prism_error, deterministic_keypair, FixtureStage, TestFixture,
};
use solana_message::Message;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;

/// Test claim at exact go_live_slot boundary → Timing boundary condition validation
///
/// **MEDIUM BUG POTENTIAL**: This test targets time comparison edge cases that could expose
/// off-by-one errors or boundary condition bugs in slot validation.
///
/// **What this tests:**
/// - Claims at exactly go_live_slot (not before, not after)
/// - Slot comparison boundary: current_slot >= go_live_slot
/// - Off-by-one errors in time comparisons (>, >=, <, <=)
/// - Clock edge cases and slot precision
///
/// **Why this is critical:**
/// Time-based validation uses slot comparisons:
/// ```rust
/// require!(
///     current_slot >= campaign.go_live_slot,
///     ErrorCode::GoLiveDateNotReached
/// );
/// ```
///
/// **Potential bugs:**
/// - Off-by-one: should be > instead of >=
/// - Race condition: slot advances during validation
/// - Clock precision issues or slot calculation errors
/// - Edge case: go_live_slot = 0 or very large values
#[test]
fn test_claim_exact_go_live_slot_boundary() {
    let mut test = TestFixture::default();

    // 1. Set up campaign through activation with custom go-live slot
    test.jump_to(FixtureStage::CohortsActivated);

    // 2. Get current slot to calculate precise go-live timing
    let current_slot = test.current_slot();
    let target_go_live_slot = current_slot + 100; // Set go-live 100 slots in the future

    println!("⏰ Current slot: {}", current_slot);
    println!("🎯 Target go-live slot: {}", target_go_live_slot);
    println!(
        "📏 Slots until go-live: {}",
        target_go_live_slot - current_slot
    );

    // 3. Activate campaign with specific go-live slot
    test.try_activate_campaign_with_args(None, Some(target_go_live_slot))
        .expect("Campaign activation should succeed");

    // 4. Get claimant and extract claim data
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    let mint = test.state.compiled_campaign.mint;
    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    let (admin, fingerprint, merkle_root, assigned_vault_index, entitlements) = {
        let (cohort, leaf) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");

        (
            test.state.compiled_campaign.admin,
            test.state.compiled_campaign.fingerprint,
            cohort.merkle_root,
            leaf.assigned_vault_index,
            leaf.entitlements,
        )
    };

    // 5. Airdrop tokens to claimant for transaction fees and ATA creation
    test.airdrop(&claimant_pubkey, 10_000_000); // 0.01 SOL

    // 6. Generate proof for claim attempts
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

    // 7. Test Phase 1: Claim BEFORE go-live slot (should fail)
    println!("\n🧪 Phase 1: Testing claim BEFORE go-live slot...");

    let pre_go_live_slot = target_go_live_slot - 5; // 5 slots before go-live
    test.warp_to_slot(pre_go_live_slot);

    let current_slot_before = test.current_slot();
    println!(
        "⏰ Current slot: {} (before go-live: {})",
        current_slot_before, target_go_live_slot
    );
    assert!(
        current_slot_before < target_go_live_slot,
        "Should be before go-live"
    );

    let (pre_claim_ix, _, _) = build_claim_tokens_v0_ix(
        &test.state.address_finder,
        admin,
        claimant_pubkey,
        mint,
        claimant_token_account,
        fingerprint,
        merkle_root,
        proof.clone(),
        assigned_vault_index,
        entitlements,
    )
    .expect("Failed to build pre-claim instruction");

    let pre_claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[pre_claim_ix.clone()], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let pre_claim_result = test.send_transaction(pre_claim_tx);

    // Should fail with GoLiveDateNotReached error
    demand_prism_error(
        pre_claim_result,
        PrismError::GoLiveDateNotReached as u32,
        "GoLiveDateNotReached",
    );
    println!("✅ Pre-go-live claim correctly failed with GoLiveDateNotReached");

    // 8. Test Phase 2: Claim EXACTLY at go-live slot (should succeed)
    println!("\n🧪 Phase 2: Testing claim EXACTLY at go-live slot...");

    test.warp_to_slot(target_go_live_slot);

    let current_slot_exact = test.current_slot();
    println!(
        "⏰ Current slot: {} (exactly go-live: {})",
        current_slot_exact, target_go_live_slot
    );
    assert_eq!(
        current_slot_exact, target_go_live_slot,
        "Should be exactly at go-live"
    );

    // 8a. First demonstrate the AlreadyProcessed issue (same as test_claim_before_go_live.rs)
    println!("🔄 Attempting to retry the exact same transaction (real-world anti-pattern)...");

    let retry_same_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[pre_claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let retry_result = test.send_transaction(retry_same_tx);

    match retry_result {
        Ok(_) => {
            println!("✅ Retry succeeded (unexpected!)");
        }
        Err(failed_meta) => {
            println!("❌ Retry failed as expected: {:?}", failed_meta.err);
            println!("   This demonstrates the duplicate transaction issue users face");
        }
    }

    // 8b. Now show the proper fix: Add compute budget instruction to make transaction different
    println!("\n🔧 Demonstrating the proper fix: Adding compute budget to create different transaction...");

    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);

    let (exact_claim_ix, _, _) = build_claim_tokens_v0_ix(
        &test.state.address_finder,
        admin,
        claimant_pubkey,
        mint,
        claimant_token_account,
        fingerprint,
        merkle_root,
        proof.clone(),
        assigned_vault_index,
        entitlements,
    )
    .expect("Failed to build exact-timing claim instruction");

    let exact_claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[compute_budget_ix, exact_claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let exact_claim_result = test.send_transaction(exact_claim_tx);

    // Should succeed - this tests the >= comparison (not just >)
    match exact_claim_result {
        Ok(_) => {
            println!("✅ Exact go-live slot claim succeeded!");
            println!("   🔬 Confirmed current_slot >= go_live_slot validation works correctly");

            // Verify claim actually worked by checking claimant balance
            let claimant_balance = test
                .get_token_account_balance(&claimant_token_account)
                .expect("Should be able to read claimant balance");

            assert!(claimant_balance > 0, "Claimant should have received tokens");
            println!("   💰 Claimant received {} tokens", claimant_balance);
        }
        Err(failed_meta) => {
            // If this fails, it might indicate a bug in our comparison logic
            panic!(
                "❌ CRITICAL: Exact go-live slot claim failed unexpectedly: {:?}\n   This suggests a bug in slot comparison logic (should be >= not >)",
                failed_meta.err
            );
        }
    }

    // 9. Test Phase 3: Claim AFTER go-live slot (should also succeed)
    println!("\n🧪 Phase 3: Testing claim AFTER go-live slot...");

    // Use a different claimant to avoid double-claiming issues
    let second_claimant_keypair = deterministic_keypair("early_adopter_2");
    let second_claimant_pubkey = second_claimant_keypair.pubkey();
    let second_claimant_token_account =
        get_associated_token_address(&second_claimant_pubkey, &mint);

    let second_entitlements = {
        let (_, leaf) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&second_claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_2 should be in EarlyAdopters cohort");
        leaf.entitlements
    };

    let second_proof = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&second_claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_2 should be in EarlyAdopters cohort");
        cohort
            .proof_for_claimant(&second_claimant_pubkey)
            .expect("Should be able to generate proof")
    };

    test.airdrop(&second_claimant_pubkey, 10_000_000); // 0.01 SOL

    let post_go_live_slot = target_go_live_slot + 10; // 10 slots after go-live
    test.warp_to_slot(post_go_live_slot);

    let current_slot_after = test.current_slot();
    println!(
        "⏰ Current slot: {} (after go-live: {})",
        current_slot_after, target_go_live_slot
    );
    assert!(
        current_slot_after > target_go_live_slot,
        "Should be after go-live"
    );

    let (post_claim_ix, _, _) = build_claim_tokens_v0_ix(
        &test.state.address_finder,
        admin,
        second_claimant_pubkey,
        mint,
        second_claimant_token_account,
        fingerprint,
        merkle_root,
        second_proof,
        assigned_vault_index,
        second_entitlements,
    )
    .expect("Failed to build post-claim instruction");

    let post_claim_tx = Transaction::new(
        &[&second_claimant_keypair],
        Message::new(&[post_claim_ix], Some(&second_claimant_pubkey)),
        test.latest_blockhash(),
    );

    let post_claim_result = test.send_transaction(post_claim_tx);

    // Should succeed
    match post_claim_result {
        Ok(_) => {
            println!("✅ Post-go-live claim succeeded!");

            let second_claimant_balance = test
                .get_token_account_balance(&second_claimant_token_account)
                .expect("Should be able to read second claimant balance");

            assert!(
                second_claimant_balance > 0,
                "Second claimant should have received tokens"
            );
            println!(
                "   💰 Second claimant received {} tokens",
                second_claimant_balance
            );
        }
        Err(failed_meta) => {
            panic!(
                "❌ Post-go-live claim failed unexpectedly: {:?}",
                failed_meta.err
            );
        }
    }

    // 10. Test Phase 4: Edge case - go-live slot 0 boundary
    println!("\n🧪 Phase 4: Testing edge case with go-live slot 0...");

    // Create a new test fixture to test go-live slot 0
    let mut zero_test = TestFixture::default();
    zero_test.jump_to(FixtureStage::CohortsActivated);

    // Activate campaign with go-live slot 0 (immediate activation)
    zero_test
        .try_activate_campaign_with_args(None, Some(0))
        .expect("Campaign activation with slot 0 should succeed");

    let zero_claimant_keypair = deterministic_keypair("early_adopter_1");
    let zero_claimant_pubkey = zero_claimant_keypair.pubkey();
    let zero_mint = zero_test.state.compiled_campaign.mint;
    let zero_claimant_token_account =
        get_associated_token_address(&zero_claimant_pubkey, &zero_mint);

    let (
        zero_admin,
        zero_fingerprint,
        zero_merkle_root,
        zero_assigned_vault_index,
        zero_entitlements,
    ) = {
        let (cohort, leaf) = zero_test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&zero_claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");

        (
            zero_test.state.compiled_campaign.admin,
            zero_test.state.compiled_campaign.fingerprint,
            cohort.merkle_root,
            leaf.assigned_vault_index,
            leaf.entitlements,
        )
    };

    let zero_proof = {
        let (cohort, _) = zero_test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&zero_claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");
        cohort
            .proof_for_claimant(&zero_claimant_pubkey)
            .expect("Should be able to generate proof")
    };

    zero_test.airdrop(&zero_claimant_pubkey, 10_000_000); // 0.01 SOL

    // Current slot should be > 0, so claim should succeed immediately
    let current_slot_zero_test = zero_test.current_slot();
    println!(
        "⏰ Current slot: {} (go-live slot: 0)",
        current_slot_zero_test
    );

    let (zero_claim_ix, _, _) = build_claim_tokens_v0_ix(
        &zero_test.state.address_finder,
        zero_admin,
        zero_claimant_pubkey,
        zero_mint,
        zero_claimant_token_account,
        zero_fingerprint,
        zero_merkle_root,
        zero_proof,
        zero_assigned_vault_index,
        zero_entitlements,
    )
    .expect("Failed to build zero go-live claim instruction");

    let zero_claim_tx = Transaction::new(
        &[&zero_claimant_keypair],
        Message::new(&[zero_claim_ix], Some(&zero_claimant_pubkey)),
        zero_test.latest_blockhash(),
    );

    let zero_claim_result = zero_test.send_transaction(zero_claim_tx);

    match zero_claim_result {
        Ok(_) => {
            println!("✅ Go-live slot 0 claim succeeded!");

            let zero_claimant_balance = zero_test
                .get_token_account_balance(&zero_claimant_token_account)
                .expect("Should be able to read zero claimant balance");

            assert!(
                zero_claimant_balance > 0,
                "Zero go-live claimant should have received tokens"
            );
            println!(
                "   💰 Zero go-live claimant received {} tokens",
                zero_claimant_balance
            );
        }
        Err(failed_meta) => {
            panic!(
                "❌ Go-live slot 0 claim failed unexpectedly: {:?}",
                failed_meta.err
            );
        }
    }

    println!("\n🎉 Go-live slot boundary test completed successfully!");
    println!("   ❌ Pre-go-live claims correctly blocked");
    println!("   ✅ Exact go-live slot claims succeeded (>= validation working)");
    println!("   ✅ Post-go-live claims succeeded");
    println!("   ✅ Go-live slot 0 edge case handled correctly");
    println!("   🔬 Timing boundary validation working properly");

    // 🎓 KEY LEARNINGS FOR DEVELOPERS:
    //
    // 1. BOUNDARY CONDITION VALIDATION: The >= comparison is critical for allowing
    //    claims exactly at the go-live slot, not just after it.
    //
    // 2. OFF-BY-ONE PREVENTION: Using >= instead of > prevents common off-by-one
    //    errors where users can't claim at the exact advertised go-live time.
    //
    // 3. EDGE CASE HANDLING: Go-live slot 0 represents immediate activation and
    //    should work correctly with the >= comparison.
    //
    // 4. TIMING PRECISION: Slot-based timing provides precise control over when
    //    claims become available, crucial for fair token distribution.
    //
    // 5. ERROR MESSAGING: GoLiveDateNotReached provides clear feedback when
    //    claims are attempted too early.
    //
    // 6. DETERMINISTIC BEHAVIOR: Slot-based validation ensures consistent behavior
    //    across different network conditions and client timing variations.
}

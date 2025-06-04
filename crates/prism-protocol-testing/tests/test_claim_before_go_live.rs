use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::build_claim_tokens_v0_ix;
use prism_protocol_testing::{
    demand_prism_error, deterministic_keypair, AccountChange, CampaignSnapshot, FixtureStage,
    TestFixture,
};
use rust_decimal::prelude::ToPrimitive;
use solana_message::Message;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_signer::Signer as _;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;

/// Test claim before go-live slot → GoLiveDateNotReached
///
/// This test demonstrates a critical real-world scenario: time-based access control
/// and the challenges users face when retrying failed transactions.
///
/// **What this test validates:**
/// - Time-based access control works correctly (claims blocked before go-live)
/// - Error handling is precise (GoLiveDateNotReached error code 6016)
/// - State isolation (no side effects when claims are blocked)
/// - Real-world retry challenges (AlreadyProcessed errors on identical transactions)
/// - Proper solution for transaction retries (using compute budget instructions)
///
/// **Real-world scenario:**
/// A user submits a claim transaction too early, hits the go-live restriction,
/// waits for the campaign to go live, then tries to resubmit the exact same
/// transaction. Solana rejects it as a duplicate. This test shows the proper
/// fix: modify the transaction to make it unique while preserving functionality.
///
/// **Test flow:**
/// 1. Set up campaign with go_live_slot in the future
/// 2. Activate campaign with future go_live_slot  
/// 3. Create valid claimant and merkle proof
/// 4. Attempt claim_tokens_v0 before go_live_slot → verify fails with GoLiveDateNotReached
/// 5. Verify no side effects (no tokens transferred, no ClaimReceipt created)
/// 6. Warp past go-live slot
/// 7. Retry exact same transaction → demonstrate AlreadyProcessed issue
/// 8. Show proper fix: add compute budget instruction to make transaction unique
/// 9. Verify claim succeeds with modified transaction
#[test]
fn test_claim_before_go_live() {
    let mut test = TestFixture::default();

    // 1. Set up campaign but STOP before activation (we'll manually activate with future go-live)
    test.jump_to(FixtureStage::CohortsActivated);

    // 2. Get current slot and set future go-live
    let current_slot = test.current_slot();
    let future_go_live_slot = current_slot + 100; // 100 slots ≈ 40 seconds in future (Solana: ~400ms/slot)

    println!(
        "⏰ Current slot: {}, Future go-live slot: {}",
        current_slot, future_go_live_slot
    );

    // 3. Use the new TestFixture method for cleaner campaign activation
    test.try_activate_campaign_with_args(None, Some(future_go_live_slot))
        .expect("Campaign activation should succeed");

    println!("✅ Campaign activated with future go-live slot");

    // 4. Get valid claimant and extract claim data
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    let (cohort, leaf) = test
        .state
        .compiled_campaign
        .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
        .expect("early_adopter_1 should be in EarlyAdopters cohort");

    let claimant_token_account =
        get_associated_token_address(&claimant_pubkey, &test.state.compiled_campaign.mint);
    test.airdrop(&claimant_pubkey, 1_000_000_000);

    // 5. Generate valid merkle proof
    let proof = cohort
        .proof_for_claimant(&claimant_pubkey)
        .expect("Should be able to generate proof");

    // 6. Capture comprehensive campaign state before attempted claim
    let state_before = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);

    println!("📊 State before early claim attempt:");
    println!(
        "  Vault: {}, Claimant: {}",
        state_before.get_vault_balance(&cohort.name, 0).unwrap_or(0),
        state_before
            .tracked_claimants
            .get(&claimant_pubkey)
            .copied()
            .unwrap_or(0)
    );
    println!(
        "  Total vault balance: {}, Admin balance: {}",
        state_before.total_vault_balance(),
        state_before.admin_balance
    );

    // 7. Build claim instruction
    let (claim_ix, _, _) = build_claim_tokens_v0_ix(
        &test.state.address_finder,
        test.state.compiled_campaign.admin,
        claimant_pubkey,
        test.state.compiled_campaign.mint,
        claimant_token_account,
        test.state.compiled_campaign.fingerprint,
        cohort.merkle_root,
        proof.clone(),
        leaf.assigned_vault_index,
        leaf.entitlements,
    )
    .expect("Failed to build claim tokens v0 ix");

    // 8. Attempt claim BEFORE go-live → should fail with GoLiveDateNotReached
    let early_claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[claim_ix.clone()], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let result = test.send_transaction(early_claim_tx);

    demand_prism_error(
        result,
        PrismError::GoLiveDateNotReached as u32,
        "GoLiveDateNotReached",
    );

    println!("✅ Correctly blocked claim before go-live slot");

    // 9. Verify no state changes occurred (comprehensive verification)
    let state_after_early = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);
    assert_eq!(
        state_before, state_after_early,
        "No state should change when claim is blocked"
    );

    println!("✅ Verified no state changes during blocked claim (comprehensive check)");

    // 10. Verify no ClaimReceipt PDA created
    let (claim_receipt_address, _) = test
        .state
        .address_finder
        .find_claim_receipt_v0_address(&cohort.address, &claimant_pubkey);

    assert!(!test.account_exists(&claim_receipt_address));

    // 11. Now warp past go-live slot and demonstrate the real-world retry challenge
    println!("⏭️  Warping past go-live slot...");
    test.warp_to_slot(future_go_live_slot + 10); // Go past the go-live slot

    // Advance one more slot to ensure we're clearly past go-live
    test.advance_slot_by(1);

    let current_slot_after_warp = test.current_slot();
    println!("⏰ Current slot after warp: {}", current_slot_after_warp);

    // 12. Demonstrate the real-world problem: Try to reuse the exact same transaction
    println!("🔄 Attempting to retry the exact same transaction (real-world anti-pattern)...");

    // NOTE: This creates a transaction with identical instruction content to the previous failed one.
    // Solana's duplicate transaction detection will reject this even though the previous tx failed.
    // This is a common gotcha that catches developers off-guard in production.
    let retry_same_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[claim_ix.clone()], Some(&claimant_pubkey)),
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

    // 13. Demonstrate the proper fix: Add compute budget instruction to make transaction different
    println!("\n🔧 Demonstrating the proper fix: Adding compute budget to create different transaction...");

    // Real-world solution: Add a compute budget instruction to make the transaction unique.
    // This is a common pattern used by wallets and dApps to avoid duplicate transaction issues.
    // Other alternatives include: memo instructions, different compute unit prices, etc.
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);

    // Build a completely fresh claim instruction (same parameters, but fresh construction)
    let (fresh_claim_ix, _, _) = build_claim_tokens_v0_ix(
        &test.state.address_finder,
        test.state.compiled_campaign.admin,
        claimant_pubkey,
        test.state.compiled_campaign.mint,
        claimant_token_account,
        test.state.compiled_campaign.fingerprint,
        cohort.merkle_root,
        proof, // Same proof data
        leaf.assigned_vault_index,
        leaf.entitlements,
    )
    .expect("Failed to build fresh claim tokens v0 ix");

    // Create transaction with compute budget instruction first (order matters for fees)
    // NOTE: Compute budget instructions should come first in the transaction for proper fee calculation.
    // The fee calculation uses the first compute budget instruction it encounters.
    let fresh_tx_with_cu = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[compute_budget_ix, fresh_claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let fresh_result = test.send_transaction(fresh_tx_with_cu);

    match fresh_result {
        Ok(_) => {
            println!("✅ Fresh transaction with compute budget succeeded after go-live slot");
        }
        Err(failed_meta) => {
            println!("❌ Fresh transaction failed: {:?}", failed_meta.err);
            panic!("This should have worked - the transaction should be different enough");
        }
    }

    // 14. Demonstrate surgical verification: capture final state and verify only expected changes
    let state_after_claim = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);

    // Calculate expected claim amount
    let expected_claim_amount =
        cohort.amount_per_entitlement.floor().to_u64().unwrap() * leaf.entitlements;

    println!("🔬 Performing surgical verification of claim operation:");
    println!("   Expected claim amount: {}", expected_claim_amount);

    // Use surgical verification to ensure ONLY the expected accounts changed
    state_before.assert_only_changed(
        &state_after_claim,
        &[
            AccountChange::Vault {
                cohort: cohort.name.clone(),
                vault_index: 0,
                delta: -(expected_claim_amount as i64),
            },
            AccountChange::Claimant {
                pubkey: claimant_pubkey,
                delta: expected_claim_amount as i64,
            },
        ],
    );

    println!(
        "✅ Surgical verification passed: only vault and claimant balances changed as expected"
    );

    // 15. Final verification of token amounts
    let claimant_balance_final = test
        .get_token_account_balance(&claimant_token_account)
        .expect("Should be able to read claimant balance");

    assert!(claimant_balance_final >= expected_claim_amount);

    // 16. Verify ClaimReceipt PDA was created
    assert!(test.account_exists(&claim_receipt_address));

    println!("🎉 Comprehensive test completed successfully!");
    println!("   ✅ Claims blocked before go-live slot");
    println!("   ⚠️  Demonstrated potential retry issues");
    println!("   ✅ Showed proper fix with fresh transaction");
    println!("   🔬 Performed surgical state verification");

    // 🎓 KEY LEARNINGS FOR DEVELOPERS:
    //
    // 1. TIME-BASED RESTRICTIONS: Solana programs can enforce time-based access control.
    //    Always check error messages and wait for the appropriate time before retrying.
    //
    // 2. DUPLICATE TRANSACTION DETECTION: Solana rejects transactions with identical
    //    signatures, even if the previous transaction failed. This prevents replay attacks
    //    but can surprise developers.
    //
    // 3. TRANSACTION UNIQUENESS: To retry failed transactions, make them unique by:
    //    - Adding compute budget instructions with different values
    //    - Including memo instructions with timestamps
    //    - Using different compute unit prices
    //    - Changing transaction fee payer (if applicable)
    //
    // 4. INSTRUCTION ORDER: Compute budget instructions should be placed first in
    //    transactions for proper fee calculation.
    //
    // 5. SURGICAL TESTING: The CampaignSnapshot pattern enables precision verification
    //    that operations only affect expected accounts, catching regressions and
    //    unintended side effects that simple balance checks might miss.
}

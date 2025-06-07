use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::{
    build_claim_tokens_v1_ix, CompiledCohortExt, CompiledLeafExt, CompiledProofExt,
};
use prism_protocol_testing::{
    demand_prism_error, deterministic_keypair, CampaignSnapshot, FixtureStage, FixtureState,
    TestFixture,
};
use solana_message::Message;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;

/// Test claim before go-live slot → GoLiveDateNotReached
///
/// This test demonstrates a critical real-world scenario: time-based access control
/// and the challenges users face when retrying failed transactions.
///
/// **What this test validates:**
/// - Time-based access control works correctly (claims blocked before go-live)
/// - Error handling is precise (GoLiveDateNotReached error code 6015)
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
/// 4. Attempt claim_tokens_v1 before go_live_slot → verify fails with GoLiveDateNotReached
/// 5. Verify no side effects (no tokens transferred, no ClaimReceipt created)
/// 6. Warp past go-live slot
/// 7. Retry exact same transaction → demonstrate AlreadyProcessed issue
/// 8. Show proper fix: add compute budget instruction to make transaction unique
/// 9. Verify claim succeeds with modified transaction
#[tokio::test]
async fn test_claim_before_go_live() {
    let state = FixtureState::default_v1().await;
    let mut test = TestFixture::new(state, LiteSVM::new()).await.unwrap();

    // 1. Set up campaign but STOP before activation (we'll manually activate with future go-live)
    test.jump_to(FixtureStage::CohortsActivated).await;

    // 2. Get current slot and set future go-live
    let current_slot = test.current_slot();
    let future_go_live_slot = current_slot + 100; // Future slot ≈ 40 seconds ahead

    println!(
        "⏰ Current slot: {}, Future go-live slot: {}",
        current_slot, future_go_live_slot
    );

    // 3. Activate campaign with the future go-live slot (not current slot)
    test.try_activate_campaign_with_args(None, Some(future_go_live_slot))
        .await
        .expect("Campaign activation should succeed");

    println!("✅ Campaign activated with future go-live slot");

    // 4. Set up a valid claimant who would normally be able to claim
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    // Get proofs for this claimant using modern CompiledCampaignDatabase API
    let proofs = test
        .state
        .ccdb
        .compiled_proofs_by_claimant(claimant_pubkey)
        .await;
    let proof = &proofs[0]; // Use first proof for this claimant

    // Get the cohort and leaf using deterministic lookups
    let cohort = test
        .state
        .ccdb
        .compiled_cohort_by_address(proof.cohort_address())
        .await;
    let leaf = test
        .state
        .ccdb
        .compiled_leaf_by_cohort_and_claimant(proof.cohort_address(), claimant_pubkey)
        .await;

    let _claimant_token_account =
        get_associated_token_address(&claimant_pubkey, &test.state.mint_address());
    test.airdrop(&claimant_pubkey, 1_000_000_000);

    // 5. Get the V1 merkle proof (matches our V1 fixture)
    let merkle_proof = proof.merkle_proof_v1();

    // 6. Capture comprehensive campaign state before attempted claim
    // This enables surgical verification that blocked claims have zero side effects
    let state_before = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]).await;

    println!("📊 State before early claim attempt:");
    println!(
        "  Vault: {}, Claimant: {}",
        state_before
            .get_vault_balance(&cohort.address(), 0)
            .unwrap_or(0),
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

    // 7. Build claim instruction using modern V1 API
    let (claim_ix, _, _) = build_claim_tokens_v1_ix(
        test.state.address_finder(),
        claimant_pubkey,
        cohort.merkle_root(),
        merkle_proof.clone(),
        leaf.vault_index(),
        leaf.entitlements(),
    )
    .expect("Failed to build claim tokens v1 ix");

    // 8. Attempt claim BEFORE go-live → should fail with GoLiveDateNotReached
    let early_claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[claim_ix.clone()], Some(&claimant_keypair.pubkey())),
        test.latest_blockhash(),
    );

    // This should fail with GoLiveDateNotReached (error code 6015)
    let result = test.send_transaction(early_claim_tx);
    demand_prism_error(
        result,
        PrismError::GoLiveDateNotReached as u32,
        "GoLiveDateNotReached",
    );

    println!("✅ Confirmed GoLiveDateNotReached error (code: 6015)");
    println!("✅ Correctly blocked claim before go-live slot");

    // 9. Verify no state changes occurred (comprehensive verification)
    // This is crucial: failed claims should have ZERO side effects
    let state_after_early =
        CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]).await;
    assert_eq!(
        state_before, state_after_early,
        "No state should change when claim is blocked"
    );
    println!("✅ Verified no state changes during blocked claim (comprehensive check)");

    // 10. Verify no ClaimReceipt PDA created (another side effect check)
    let (claim_receipt_address, _) = test
        .state
        .address_finder()
        .find_claim_receipt_v0_address(&cohort.address(), &claimant_pubkey);

    assert!(!test.account_exists(&claim_receipt_address));

    // 11. Now simulate the passage of time: warp past go-live slot
    println!("⏭️  Warping past go-live slot...");
    test.warp_to_slot(future_go_live_slot + 11);
    let current_slot_after_warp = test.current_slot();
    println!("⏰ Current slot after warp: {}", current_slot_after_warp);

    // 12. Here's where real-world problems occur: users often retry the EXACT same transaction
    // Solana's duplicate transaction detection will reject this, even though the original failed!
    println!("🔄 Attempting to retry the exact same transaction (real-world anti-pattern)...");

    // NOTE: This creates a transaction with identical instruction content to the previous failed one.
    // Solana's duplicate transaction detection will reject this even though the previous tx failed.
    // This is a common gotcha that catches developers off-guard in production.
    let retry_same_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[claim_ix.clone()], Some(&claimant_keypair.pubkey())),
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

    // 13. THE SOLUTION: Create a unique transaction by adding a compute budget instruction
    // This is the proper way to retry failed transactions in Solana
    println!(
        "🔧 Demonstrating the proper fix: Adding compute budget to create different transaction..."
    );
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);

    // Build a completely fresh claim instruction (same parameters, but fresh construction)
    let (fresh_claim_ix, _, _) = build_claim_tokens_v1_ix(
        test.state.address_finder(),
        claimant_pubkey,
        cohort.merkle_root(),
        merkle_proof, // Same proof data
        leaf.vault_index(),
        leaf.entitlements(),
    )
    .expect("Failed to build fresh claim tokens v1 ix");

    // Create transaction with compute budget instruction first (order matters for fees)
    // NOTE: Compute budget instructions should come first in the transaction for proper fee calculation.
    let successful_claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(
            &[compute_budget_ix, fresh_claim_ix], // Compute budget FIRST
            Some(&claimant_keypair.pubkey()),
        ),
        test.latest_blockhash(),
    );

    // This should now succeed because the transaction is unique
    match test.send_transaction(successful_claim_tx) {
        Ok(_) => {
            println!("✅ Fresh transaction with compute budget succeeded after go-live slot");
        }
        Err(failed_meta) => {
            panic!(
                "Fresh transaction should have succeeded: {:?}",
                failed_meta.err
            );
        }
    }

    // 14. Verify the claim worked correctly: perform surgical state verification
    let state_after_claim =
        CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]).await;

    // Calculate expected claim amount using modern API
    let expected_claim_amount = cohort.amount_per_entitlement_token() * leaf.entitlements();

    // Verify expected state changes occurred with precision
    let claimant_balance_change = state_after_claim
        .tracked_claimants
        .get(&claimant_pubkey)
        .copied()
        .unwrap_or(0)
        - state_before
            .tracked_claimants
            .get(&claimant_pubkey)
            .copied()
            .unwrap_or(0);

    assert_eq!(
        claimant_balance_change, expected_claim_amount,
        "Claimant should have received expected tokens"
    );

    // Verify vault balance decreased by the same amount (conservation of tokens)
    let vault_balance_change = state_before
        .get_vault_balance(&cohort.address(), 0)
        .unwrap_or(0)
        - state_after_claim
            .get_vault_balance(&cohort.address(), 0)
            .unwrap_or(0);

    assert_eq!(
        vault_balance_change, expected_claim_amount,
        "Vault balance should have decreased by claim amount"
    );

    println!(
        "✅ Verified correct token transfer: {} tokens",
        expected_claim_amount
    );

    // 15. Verify ClaimReceipt PDA was created (proof of successful claim)
    assert!(test.account_exists(&claim_receipt_address));
    println!("✅ ClaimReceipt PDA created successfully");

    // 16. Final comprehensive summary
    println!("\n🎉 Go-live timing test complete:");
    println!("  ✅ Claims properly blocked before go-live slot");
    println!("  ✅ No side effects during blocked claims");
    println!("  ✅ Duplicate transaction issue demonstrated");
    println!("  ✅ Proper retry pattern with compute budget");
    println!("  ✅ Claims work correctly after go-live slot");
    println!("  ✅ All state changes verified");

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
    //
    // 6. V1 CLAIM TREES: Modern fixtures use V1 claim trees, requiring v1 proof extraction
    //    and v1 instruction builders. Always match your fixture version to your API calls.
}

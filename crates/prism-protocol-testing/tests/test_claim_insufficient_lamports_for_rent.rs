use prism_protocol_sdk::build_claim_tokens_v0_ix;
use prism_protocol_testing::{deterministic_keypair, CampaignSnapshot, FixtureStage, TestFixture};
use solana_instruction;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error;
use spl_associated_token_account::get_associated_token_address;

/// Test claim when claimant has insufficient lamports for ATA rent → Account creation failure
///
/// **HIGH BUG POTENTIAL**: This test targets `init_if_needed` edge cases that could expose
/// subtle bugs in account initialization logic and rent calculation.
///
/// **What this tests:**
/// - Account initialization when claimant has insufficient SOL for rent exemption
/// - Proper error handling vs partial transaction success  
/// - State consistency when account creation fails mid-transaction
/// - Rent calculation edge cases for Associated Token Accounts
///
/// **Why this is critical:**
/// The `init_if_needed` constraint in claim_tokens_v0 creates an ATA for the claimant if it
/// doesn't exist. This involves:
/// 1. Rent calculation for new account
/// 2. SOL deduction from claimant
/// 3. Account creation with proper ownership
///
/// Edge cases that could expose bugs:
/// - Claimant has EXACTLY enough SOL for tx fees but not rent
/// - Claimant has partial SOL (enough for fees, not enough for rent)
/// - Race conditions where multiple claims try to init same ATA
/// - Rent exemption threshold changes during transaction
///
/// **Test Strategy:**
/// 1. Set up valid claim scenario
/// 2. Drain claimant SOL to precise levels (just tx fees, partial rent, etc.)
/// 3. Attempt claim → should fail gracefully with proper error
/// 4. Verify no partial state corruption (no partial account creation)
/// 5. Verify vault balances unchanged
///
/// **Expected behavior:** Clean failure with account creation error, no state corruption
#[test]
fn test_claim_insufficient_lamports_for_rent() {
    let mut test = TestFixture::default();

    // 1. Set up active campaign (past go-live)
    test.jump_to(FixtureStage::CampaignActivated);
    test.advance_slot_by(20); // Past go-live slot

    // 2. Get claimant who will have insufficient rent
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    // 3. Extract claim data
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

    // 4. Calculate rent requirements for ATA creation
    // Token accounts are 165 bytes according to get_token_account_balance
    // Rent exemption for 165-byte account is approximately 2,039,280 lamports on mainnet
    // Using a conservative estimate for testing
    const ESTIMATED_ATA_RENT_REQUIREMENT: u64 = 2_500_000; // ~0.0025 SOL, conservative estimate

    // The claim instruction ALSO creates a ClaimReceipt PDA account
    // From the logs we can see it needs ~1,677,360 lamports
    const ESTIMATED_CLAIM_RECEIPT_RENT_REQUIREMENT: u64 = 2_000_000; // ~0.002 SOL, conservative estimate

    // Total rent needed for both accounts
    const TOTAL_ESTIMATED_RENT_REQUIREMENT: u64 =
        ESTIMATED_ATA_RENT_REQUIREMENT + ESTIMATED_CLAIM_RECEIPT_RENT_REQUIREMENT;

    println!(
        "💰 Estimated ATA rent requirement: {} lamports",
        ESTIMATED_ATA_RENT_REQUIREMENT
    );
    println!(
        "💰 Estimated ClaimReceipt rent requirement: {} lamports",
        ESTIMATED_CLAIM_RECEIPT_RENT_REQUIREMENT
    );
    println!(
        "💰 Total estimated rent requirement: {} lamports",
        TOTAL_ESTIMATED_RENT_REQUIREMENT
    );

    // 5. Calculate rough transaction fee (conservative estimate)
    let estimated_tx_fee = 10_000; // ~0.00001 SOL for tx fees

    // 6. Give claimant insufficient lamports for rent
    // Strategy: Give enough for tx fees but NOT enough for ATA rent
    let insufficient_balance = estimated_tx_fee + (ESTIMATED_ATA_RENT_REQUIREMENT / 2); // Only half the needed ATA rent
    test.airdrop(&claimant_pubkey, insufficient_balance);

    println!(
        "⚖️  Claimant balance: {} lamports (insufficient for {} total rent)",
        insufficient_balance, TOTAL_ESTIMATED_RENT_REQUIREMENT
    );

    // 7. Verify ATA does not exist yet
    assert!(
        !test.account_exists(&claimant_token_account),
        "ATA should not exist before claim attempt"
    );

    // 8. Capture state before attempted claim
    let state_before_claim = CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);

    let vault_address = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");

        cohort.vaults[assigned_vault_index as usize].address
    };

    let vault_balance_before = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    println!("📊 State before insufficient rent claim attempt:");
    println!("  Vault balance: {}", vault_balance_before);
    println!(
        "  ATA exists: {}",
        test.account_exists(&claimant_token_account)
    );

    // 9. Generate proof for claim attempt
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

    // 10. Attempt claim with insufficient rent → should fail
    println!("🔄 Attempting claim with insufficient lamports for ATA rent...");

    let (claim_ix, _, _) = build_claim_tokens_v0_ix(
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
    .expect("Failed to build claim instruction");

    let claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let claim_result = test.send_transaction(claim_tx);

    // 11. Verify claim failed with appropriate error
    match claim_result {
        Ok(_) => {
            panic!("❌ Claim should have failed due to insufficient lamports for ATA rent!");
        }
        Err(failed_meta) => {
            println!("✅ Claim correctly failed: {:?}", failed_meta.err);

            // The System Program returns Custom(1) for insufficient lamports for rent
            // This is exactly what we expect when ATA creation fails due to insufficient rent
            match failed_meta.err {
                solana_transaction_error::TransactionError::InstructionError(
                    _,
                    solana_instruction::error::InstructionError::Custom(1),
                ) => {
                    println!("✅ Confirmed insufficient lamports for rent error (System Program Custom(1))");
                }
                _ => {
                    // Also check for other potential rent-related errors
                    let error_str = format!("{:?}", failed_meta.err);
                    let is_rent_related = error_str.contains("InsufficientFunds")
                        || error_str.contains("insufficient")
                        || error_str.contains("rent")
                        || error_str.contains("lamports");

                    if !is_rent_related {
                        panic!(
                            "Expected rent-related error (preferably System Program Custom(1)), got: {:?}",
                            failed_meta.err
                        );
                    }
                    println!(
                        "✅ Confirmed other rent-related error: {:?}",
                        failed_meta.err
                    );
                }
            }
        }
    }

    // 12. Verify no state changes occurred during failed attempt
    let state_after_failed_claim =
        CampaignSnapshot::capture_with_claimants(&test, &[claimant_pubkey]);

    assert_eq!(
        state_before_claim, state_after_failed_claim,
        "No state should change when claim fails due to insufficient rent"
    );

    // 13. Verify ATA was not created
    assert!(
        !test.account_exists(&claimant_token_account),
        "ATA should not exist after failed rent payment"
    );

    // 14. Verify vault balance unchanged
    let vault_balance_after = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    assert_eq!(
        vault_balance_before, vault_balance_after,
        "Vault balance should be unchanged after failed claim"
    );

    println!("✅ Verified no state corruption during insufficient rent failure");

    // 15. BONUS: Verify claim succeeds when claimant has sufficient rent
    println!("\n🎯 Bonus verification: Claim succeeds with sufficient rent...");

    // Use a different claimant to avoid ClaimReceipt PDA conflicts
    let bonus_claimant_keypair = deterministic_keypair("early_adopter_2");
    let bonus_claimant_pubkey = bonus_claimant_keypair.pubkey();
    let bonus_claimant_token_account = get_associated_token_address(&bonus_claimant_pubkey, &mint);

    // Extract claim data for the new claimant
    let (bonus_assigned_vault_index, bonus_entitlements) = {
        let (_cohort, leaf) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&bonus_claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_2 should be in EarlyAdopters cohort");

        (leaf.assigned_vault_index, leaf.entitlements)
    };

    // Give bonus claimant sufficient lamports for both rent and fees
    let sufficient_balance = TOTAL_ESTIMATED_RENT_REQUIREMENT + estimated_tx_fee + 100_000; // Extra buffer
    test.airdrop(&bonus_claimant_pubkey, sufficient_balance);

    // Generate proof for the bonus claimant
    let bonus_proof = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&bonus_claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_2 should be in EarlyAdopters cohort");
        cohort
            .proof_for_claimant(&bonus_claimant_pubkey)
            .expect("Should be able to generate proof")
    };

    let (success_claim_ix, _, _) = build_claim_tokens_v0_ix(
        &test.state.address_finder,
        admin,
        bonus_claimant_pubkey,
        mint,
        bonus_claimant_token_account,
        fingerprint,
        merkle_root,
        bonus_proof,
        bonus_assigned_vault_index,
        bonus_entitlements,
    )
    .expect("Failed to build success claim instruction");

    let success_claim_tx = Transaction::new(
        &[&bonus_claimant_keypair],
        Message::new(&[success_claim_ix], Some(&bonus_claimant_pubkey)),
        test.latest_blockhash(),
    );

    test.send_transaction(success_claim_tx)
        .expect("Claim should succeed with sufficient rent");

    // Verify ATA was created and claim succeeded
    assert!(
        test.account_exists(&bonus_claimant_token_account),
        "ATA should exist after successful claim with sufficient rent"
    );

    let final_vault_balance = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    assert!(
        final_vault_balance < vault_balance_before,
        "Vault balance should decrease after successful claim"
    );

    println!("✅ Bonus verification passed: Sufficient rent enables successful claim");

    println!("🎉 Insufficient lamports for rent test completed successfully!");
    println!("   ❌ Insufficient rent correctly blocked claim");
    println!("   🔬 Verified no state corruption during failure");
    println!("   ✅ Sufficient rent enabled successful claim");

    // 🎓 KEY LEARNINGS FOR DEVELOPERS:
    //
    // 1. RENT EXEMPTION REQUIREMENTS: ATA creation requires precise rent calculations.
    //    The `init_if_needed` constraint fails gracefully when insufficient rent is available.
    //
    // 2. ATOMIC TRANSACTION BEHAVIOR: Failed account initialization does not partially
    //    corrupt state. The entire transaction fails atomically.
    //
    // 3. ERROR HANDLING PATTERNS: Rent failures produce deterministic errors that can
    //    be distinguished from business logic failures.
    //
    // 4. DEFENSIVE PROGRAMMING: Always verify sufficient balance before operations that
    //    create accounts. Consider providing clear error messages for rent failures.
    //
    // 5. TESTING EDGE CASES: Boundary testing (insufficient vs sufficient rent) reveals
    //    the exact behavior of complex constraint combinations like `init_if_needed`.
}

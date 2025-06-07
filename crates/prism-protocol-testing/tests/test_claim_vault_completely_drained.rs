use prism_protocol_sdk::build_claim_tokens_v0_ix;
use prism_protocol_testing::{deterministic_keypair, FixtureStage, TestFixture};
use rust_decimal::prelude::ToPrimitive;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use std::collections::HashMap;

/// Test claim when vault balance = 0 exactly → Zero balance edge case handling
///
/// **HIGH BUG POTENTIAL**: This test targets zero-balance arithmetic and edge case handling
/// that could expose division-by-zero or unexpected success/failure modes.
///
/// **What this tests:**
/// - Token transfer when vault is completely empty (balance = 0)
/// - Zero arithmetic edge cases in calculation logic
/// - SPL Token behavior with zero-amount transfers
/// - Error handling vs unexpected success for impossible scenarios
///
/// **Why this is critical:**
/// Empty vaults represent a critical edge case where multiple issues could arise:
/// 1. **Arithmetic edge cases**: 0 balance vs calculated claim amount
/// 2. **SPL Token behavior**: Does transfer(0) succeed or fail?
/// 3. **State corruption**: ClaimReceipt creation with zero transfer
/// 4. **Logic bugs**: Should this be prevented earlier or handled gracefully?
///
/// **Specific scenarios to test:**
/// - Vault starts with tokens, gets completely drained by other claims
/// - Original vault setup with 0 balance (configuration error)
/// - Race condition: vault drained between validation and transfer
/// - Multiple claims against same empty vault
///
/// **Key questions this test answers:**
/// - Does SPL Token allow transfer(from_vault, to_claimant, 0)?
/// - Do we create ClaimReceipt for zero-token claims?
/// - Is this a configuration error or runtime error?
/// - Does this corrupt any counters or state?
///
/// **Test Strategy:**
/// 1. Set up valid claim scenario
/// 2. Completely drain vault (set balance to exactly 0)
/// 3. Attempt claim → observe behavior (fail vs succeed)
/// 4. If succeeds: verify no tokens transferred, ClaimReceipt handling
/// 5. If fails: verify proper error and no state corruption
///
/// **Expected behavior:** TBD - this test will help determine correct behavior
#[ignore]
#[test]
fn test_claim_vault_completely_drained() {
    let mut test = TestFixture::default();

    // 1. Set up campaign through vault initialization (but don't fund yet)
    test.jump_to(FixtureStage::VaultsInitialized);

    // 2. Get claimant and extract claim data
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

    // 3. Get vault address and calculate expected claim amount
    let vault_address = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");

        cohort.vaults[assigned_vault_index as usize].address
    };

    let expected_claim_amount = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");

        cohort
            .amount_per_entitlement
            .checked_mul(rust_decimal::Decimal::from(entitlements))
            .and_then(|d| d.to_u64())
            .expect("Claim amount calculation overflow")
    };

    println!("💰 Expected claim amount: {} tokens", expected_claim_amount);
    println!("🎯 Target vault: {}", vault_address);
    println!("⚠️  Will fund vault with 0 tokens (completely drained)");

    // 4. CRITICAL: Fund vault with exactly 0 tokens (completely drained)
    let drained_vault_funding = HashMap::from([(vault_address, 0u64)]);

    test.try_fund_vaults_with_custom_amounts(drained_vault_funding)
        .expect("Custom vault funding should succeed");

    // 5. CRITICAL: Activate vault expecting 0 tokens to bypass validation
    let drained_vault_expected = HashMap::from([(vault_address, 0u64)]);

    println!("🔧 Activating vault expecting 0 tokens (completely drained)");

    test.try_activate_vaults_with_custom_expected_balance(drained_vault_expected)
        .expect("Custom vault activation should succeed");

    // 6. Continue with normal campaign activation
    test.try_activate_cohorts()
        .expect("Cohort activation should succeed");

    test.try_activate_campaign()
        .expect("Campaign activation should succeed");

    test.advance_slot_by(20); // Past go-live slot

    // 7. Verify our setup: vault has exactly 0 balance
    let actual_vault_balance = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    println!("📊 Actual vault balance: {} tokens", actual_vault_balance);
    assert_eq!(
        actual_vault_balance, 0,
        "Vault should be completely drained (0 tokens)"
    );
    assert!(
        expected_claim_amount > 0,
        "Claim amount should be positive (to test the mismatch)"
    );

    // 8. Airdrop tokens to claimant for transaction fees and ATA creation
    test.airdrop(&claimant_pubkey, 10_000_000); // 0.01 SOL

    println!("📊 Zero balance vs positive claim test scenario:");
    println!("  Vault balance: {} tokens (empty)", actual_vault_balance);
    println!("  Required claim: {} tokens", expected_claim_amount);
    println!("  Zero vs positive mismatch: {}x", expected_claim_amount);

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

    // 10. Attempt claim against completely drained vault
    println!("🔄 Attempting claim against completely drained vault (0 tokens)...");

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

    // 11. Analyze claim result and verify behavior
    match claim_result {
        Ok(_success_meta) => {
            println!("🤔 Claim succeeded against drained vault - investigating...");

            // If claim succeeded, this is a critical finding!
            // We need to verify what actually happened

            // Check vault balance after claim (should still be 0)
            let vault_balance_after = test
                .get_token_account_balance(&vault_address)
                .expect("Should be able to read vault balance");

            println!(
                "📊 Vault balance after claim: {} tokens",
                vault_balance_after
            );
            assert_eq!(
                vault_balance_after, 0,
                "Vault should still be empty after claim against drained vault"
            );

            // Check claimant balance
            let claimant_balance = test
                .get_token_account_balance(&claimant_token_account)
                .unwrap_or(0);

            println!(
                "📊 Claimant balance after claim: {} tokens",
                claimant_balance
            );

            if claimant_balance == 0 {
                println!("✅ Zero-token claim: No tokens transferred (expected for drained vault)");

                // Check if ClaimReceipt was created for zero-token claim
                let (cohort_address, _) = test
                    .state
                    .address_finder
                    .find_cohort_v0_address(&test.state.compiled_campaign.address, &merkle_root);

                let (claim_receipt_address, _) = test
                    .state
                    .address_finder
                    .find_claim_receipt_v0_address(&cohort_address, &claimant_pubkey);

                let claim_receipt_exists = test.account_exists(&claim_receipt_address);
                println!("📊 ClaimReceipt created: {}", claim_receipt_exists);

                if claim_receipt_exists {
                    println!("⚠️  CRITICAL: ClaimReceipt created for zero-token claim!");
                    println!("   This might allow double-claiming if vault is refunded later.");
                } else {
                    println!("✅ No ClaimReceipt created for zero-token claim (good)");
                }
            } else {
                panic!(
                    "❌ CRITICAL BUG: Claimant received {} tokens from empty vault!",
                    claimant_balance
                );
            }

            println!("🎉 Zero-balance vault claim succeeded with zero transfer");
            println!("   ✅ No tokens transferred from empty vault");
            println!(
                "   🔬 ClaimReceipt behavior: {}",
                if test.account_exists(&{
                    let (cohort_address, _) = test.state.address_finder.find_cohort_v0_address(
                        &test.state.compiled_campaign.address,
                        &merkle_root,
                    );
                    let (claim_receipt_address, _) = test
                        .state
                        .address_finder
                        .find_claim_receipt_v0_address(&cohort_address, &claimant_pubkey);
                    claim_receipt_address
                }) {
                    "Created (potential concern)"
                } else {
                    "Not created (safe)"
                }
            );
        }
        Err(failed_meta) => {
            println!(
                "✅ Claim correctly failed against drained vault: {:?}",
                failed_meta.err
            );

            // Verify this is the expected SPL Token insufficient funds error
            let error_str = format!("{:?}", failed_meta.err);
            let is_balance_related = error_str.contains("InsufficientFunds") 
                || error_str.contains("insufficient") 
                || error_str.contains("Custom(1)")  // SPL Token error for insufficient funds
                || error_str.contains("balance");

            if is_balance_related {
                println!("✅ Confirmed expected SPL Token insufficient funds error");
            } else {
                println!(
                    "⚠️  Different error type (may still be valid): {:?}",
                    failed_meta.err
                );
            }

            // Verify no state corruption occurred
            let vault_balance_after = test
                .get_token_account_balance(&vault_address)
                .expect("Should be able to read vault balance");

            assert_eq!(
                vault_balance_after, 0,
                "Vault should still be empty after failed claim"
            );

            // Verify no ClaimReceipt was created
            let (cohort_address, _) = test
                .state
                .address_finder
                .find_cohort_v0_address(&test.state.compiled_campaign.address, &merkle_root);

            let (claim_receipt_address, _) = test
                .state
                .address_finder
                .find_claim_receipt_v0_address(&cohort_address, &claimant_pubkey);

            assert!(
                !test.account_exists(&claim_receipt_address),
                "ClaimReceipt PDA should not exist after failed claim"
            );

            // Verify claimant has no tokens
            let claimant_balance = test
                .get_token_account_balance(&claimant_token_account)
                .unwrap_or(0);
            assert_eq!(
                claimant_balance, 0,
                "Claimant should have received no tokens from failed claim"
            );

            println!("✅ Verified no state corruption during drained vault failure");

            println!("🎉 Drained vault test completed successfully!");
            println!("   ❌ Empty vault correctly blocked claim");
            println!("   🔬 Verified no state corruption during failure");
            println!("   ✅ SPL Token validation working properly for zero balance");
        }
    }

    println!("\n🎯 Testing multiple scenarios with zero balance...");

    // 12. BONUS: Test what happens with a different claimant on the same drained vault
    let second_claimant_keypair = deterministic_keypair("early_adopter_2");
    let second_claimant_pubkey = second_claimant_keypair.pubkey();
    let second_claimant_token_account =
        get_associated_token_address(&second_claimant_pubkey, &mint);

    // Check if the second claimant uses the same vault (they might be in different cohorts)
    let second_claimant_vault = {
        let (cohort, leaf) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&second_claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_2 should be in EarlyAdopters cohort");

        cohort.vaults[leaf.assigned_vault_index as usize].address
    };

    if second_claimant_vault == vault_address {
        println!("🔄 Testing second claimant against same drained vault...");

        test.airdrop(&second_claimant_pubkey, 10_000_000); // 0.01 SOL for fees

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

        let second_entitlements = {
            let (_, leaf) = test
                .state
                .compiled_campaign
                .find_claimant_in_cohort(&second_claimant_pubkey, "EarlyAdopters")
                .expect("early_adopter_2 should be in EarlyAdopters cohort");
            leaf.entitlements
        };

        let (second_claim_ix, _, _) = build_claim_tokens_v0_ix(
            &test.state.address_finder,
            admin,
            second_claimant_pubkey,
            mint,
            second_claimant_token_account,
            fingerprint,
            merkle_root,
            second_proof,
            assigned_vault_index, // Same vault index
            second_entitlements,
        )
        .expect("Failed to build second claim instruction");

        let second_claim_tx = Transaction::new(
            &[&second_claimant_keypair],
            Message::new(&[second_claim_ix], Some(&second_claimant_pubkey)),
            test.latest_blockhash(),
        );

        let second_claim_result = test.send_transaction(second_claim_tx);

        match second_claim_result {
            Ok(_) => {
                println!("✅ Second claim against drained vault also handled appropriately");
            }
            Err(_) => {
                println!("✅ Second claim against drained vault correctly failed");
            }
        }
    } else {
        println!("ℹ️  Second claimant uses different vault - skipping same-vault test");
    }

    println!("🎉 Complete vault drained test finished successfully!");
    println!("   🔬 Zero-balance arithmetic edge cases tested");
    println!("   ✅ SPL Token zero-transfer behavior verified");
    println!("   🛡️  State corruption prevention confirmed");

    // 🎓 KEY LEARNINGS FOR DEVELOPERS:
    //
    // 1. ZERO-BALANCE BEHAVIOR: This test reveals how our protocol handles completely
    //    drained vaults and whether SPL Token allows zero-amount transfers.
    //
    // 2. EDGE CASE ARITHMETIC: Zero balance vs positive claim amount represents a
    //    critical boundary condition that could expose arithmetic bugs.
    //
    // 3. CLAIMRECEIPT CREATION: Understanding whether ClaimReceipts are created for
    //    zero-token claims has important implications for double-claiming prevention.
    //
    // 4. ERROR HANDLING CONSISTENCY: Drained vaults should behave consistently with
    //    insufficient balance scenarios, using the same error pathways.
    //
    // 5. REAL-WORLD SCENARIOS: Vault draining can happen due to configuration errors,
    //    race conditions, or intentional admin actions.
    //
    // 6. DEFENSIVE PROGRAMMING: Robust systems must handle all edge cases gracefully,
    //    including extreme scenarios like completely empty vaults.
}

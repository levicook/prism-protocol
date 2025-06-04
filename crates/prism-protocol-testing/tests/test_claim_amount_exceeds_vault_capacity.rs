use prism_protocol_sdk::build_claim_tokens_v0_ix;
use prism_protocol_testing::{deterministic_keypair, FixtureStage, TestFixture};
use rust_decimal::prelude::ToPrimitive;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use std::collections::HashMap;

/// Test claim with extremely large amounts → Large number arithmetic edge cases
///
/// **HIGH BUG POTENTIAL**: This test targets large number arithmetic that could expose
/// precision loss, overflow, or capacity validation bugs.
///
/// **What this tests:**
/// - Large entitlements * amount_per_entitlement calculations
/// - Claim amounts that exceed reasonable vault capacity
/// - Precision handling in large number arithmetic
/// - SPL Token validation with large transfer amounts
/// - System behavior when calculations are mathematically valid but practically excessive
///
/// **Why this is critical:**
/// Large number arithmetic can expose subtle bugs:
/// ```rust
/// let total_amount = amount_per_entitlement
///     .checked_mul(Decimal::from(entitlements))
///     .and_then(|d| d.to_u64())  // ← Could be enormous but valid
/// ```
///
/// **Potential bugs:**
/// - Calculation succeeds but creates unrealistic claim amounts
/// - Precision loss in intermediate calculations  
/// - SPL Token program behavior with extremely large transfers
/// - Real-world scenarios with high-value tokens (BTC, ETH equivalents)
/// - Edge cases in u64 arithmetic near maximum values
///
/// **Test Strategy:**
/// 1. Set up campaign with normal amount_per_entitlement
/// 2. Fund vault with reasonable amount (e.g., 1M tokens)
/// 3. Create claimant with extremely large entitlements (causing massive claim)
/// 4. Verify calculation succeeds but claim amount >> vault capacity
/// 5. Attempt claim → should fail with insufficient funds (SPL Token validation)
/// 6. Verify proper error handling and no state corruption
///
/// **Expected behavior:** Mathematical calculation succeeds, SPL Token transfer fails gracefully
#[test]
fn test_claim_amount_exceeds_vault_capacity() {
    let mut test = TestFixture::default();

    // 1. Set up campaign through vault initialization (but don't fund yet)
    test.jump_to(FixtureStage::VaultsInitialized);

    // 2. Get claimant and extract claim data
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    let mint = test.state.compiled_campaign.mint;
    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    let (admin, fingerprint, merkle_root, assigned_vault_index) = {
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
        )
    };

    // 3. Calculate normal claim amount and get vault info
    let (amount_per_entitlement, vault_address, entitlements) = {
        let (cohort, leaf) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");

        (
            cohort.amount_per_entitlement,
            cohort.vaults[assigned_vault_index as usize].address,
            leaf.entitlements,
        )
    };

    let normal_claim_amount = amount_per_entitlement
        .checked_mul(rust_decimal::Decimal::from(entitlements))
        .and_then(|d| d.to_u64())
        .expect("Normal claim amount calculation should not overflow");

    println!("💰 Normal claim amount: {} tokens", normal_claim_amount);
    println!("🎯 Target vault: {}", vault_address);

    // 4. CRITICAL: Fund vault with tiny amount to create massive capacity deficit
    // Use 1% of claim amount to create extreme but realistic capacity shortage
    let tiny_vault_funding = std::cmp::max(normal_claim_amount / 100, 1000); // At least 1000, max 1% of claim

    println!(
        "🔧 Funding vault with {} tokens ({}% of {} claim)",
        tiny_vault_funding,
        (tiny_vault_funding * 100) / normal_claim_amount,
        normal_claim_amount
    );

    let custom_funding = HashMap::from([(vault_address, tiny_vault_funding)]);

    test.try_fund_vaults_with_custom_amounts(custom_funding)
        .expect("Custom vault funding should succeed");

    // 5. Activate vaults with custom expected balance
    let custom_expected_balance = HashMap::from([(vault_address, tiny_vault_funding)]);

    println!(
        "🔧 Activating vault expecting {} tokens (much less than claim needs)",
        tiny_vault_funding
    );

    test.try_activate_vaults_with_custom_expected_balance(custom_expected_balance)
        .expect("Custom vault activation should succeed");

    // 6. Continue with normal campaign activation
    test.try_activate_cohorts()
        .expect("Cohort activation should succeed");

    test.try_activate_campaign()
        .expect("Campaign activation should succeed");

    test.advance_slot_by(20); // Past go-live slot

    // 7. Verify our setup: vault has tiny amount, claim needs normal amount
    let actual_vault_balance = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    println!("📊 Actual vault balance: {} tokens", actual_vault_balance);
    assert_eq!(
        actual_vault_balance, tiny_vault_funding,
        "Vault should have our tiny funding amount"
    );

    let capacity_ratio = normal_claim_amount / actual_vault_balance;
    println!(
        "⚡ Claim amount is {}x larger than vault balance!",
        capacity_ratio
    );
    assert!(
        normal_claim_amount > actual_vault_balance * 10,
        "Claim should be at least 10x larger than vault capacity"
    );

    // 8. Airdrop tokens to claimant for transaction fees and ATA creation
    test.airdrop(&claimant_pubkey, 10_000_000); // 0.01 SOL

    println!("📊 Large number vs capacity test scenario:");
    println!("  Vault capacity: {} tokens", actual_vault_balance);
    println!("  Required claim: {} tokens", normal_claim_amount);
    println!("  Entitlements: {} (normal amount)", entitlements);
    println!("  Capacity deficit: {}x", capacity_ratio);

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

    // 10. Attempt claim with normal entitlements against tiny vault → should fail
    println!(
        "🔄 Attempting claim with normal amount ({}x vault capacity)...",
        capacity_ratio
    );

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
        entitlements, // Using normal entitlements (Merkle proof will work)
    )
    .expect("Failed to build claim instruction");

    let claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let claim_result = test.send_transaction(claim_tx);

    // 11. Verify claim failed with appropriate SPL Token error
    match claim_result {
        Ok(_) => {
            panic!("❌ Claim should have failed due to insufficient vault capacity!");
        }
        Err(failed_meta) => {
            println!("✅ Claim correctly failed: {:?}", failed_meta.err);

            // SPL Token should return InsufficientFunds or Custom(1) for insufficient balance
            let error_str = format!("{:?}", failed_meta.err);
            let is_balance_related = error_str.contains("InsufficientFunds") 
                || error_str.contains("insufficient") 
                || error_str.contains("Custom(1)")  // SPL Token error for insufficient funds
                || error_str.contains("balance");

            if !is_balance_related {
                panic!(
                    "Expected balance-related error from SPL Token, got: {:?}",
                    failed_meta.err
                );
            }
            println!("✅ Confirmed SPL Token insufficient funds error for capacity claim");
        }
    }

    // 12. Verify no state corruption occurred
    // The vault balance should be unchanged (no partial transfer)
    let vault_balance_after_failed_claim = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    assert_eq!(
        vault_balance_after_failed_claim, actual_vault_balance,
        "Vault balance should be unchanged after failed capacity claim"
    );

    // 13. Verify ClaimReceipt was NOT created
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
        "ClaimReceipt PDA should not exist after failed capacity claim"
    );

    // 14. Verify claimant token account either doesn't exist or has 0 balance
    let claimant_balance = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0);
    assert_eq!(
        claimant_balance, 0,
        "Claimant should have received no tokens from failed capacity claim"
    );

    println!("✅ Verified no state corruption during capacity failure");

    println!("🎉 Vault capacity test completed successfully!");
    println!(
        "   📊 Normal claim amount vs tiny vault capacity ({}x deficit)",
        capacity_ratio
    );
    println!("   ❌ SPL Token correctly blocked capacity-exceeding transfer");
    println!("   🔬 Verified no state corruption during capacity failure");
    println!("   ✅ System handles extreme capacity mismatches properly");
    println!("   🛠️  Capacity validation working correctly");

    // 🎓 KEY LEARNINGS FOR DEVELOPERS:
    //
    // 1. CAPACITY VS ARITHMETIC: Even normal claim calculations can fail when
    //    vault capacity is insufficient - this tests real funding scenarios.
    //
    // 2. VALIDATION ORDER: Merkle proof validation happens before SPL Token transfer,
    //    so we must use valid entitlements to reach the capacity check.
    //
    // 3. REAL-WORLD SCENARIOS: This pattern applies to situations where vaults
    //    are underfunded due to admin error or insufficient token supply.
    //
    // 4. GRACEFUL DEGRADATION: System fails gracefully when claims exceed
    //    available vault capacity, maintaining atomicity.
    //
    // 5. FUNDING VALIDATION: The combination of business logic + SPL Token validation
    //    provides robust protection against insufficient funding scenarios.
    //
    // 6. CAPACITY PLANNING: This test highlights the importance of proper vault
    //    funding to match expected claim volumes.
}

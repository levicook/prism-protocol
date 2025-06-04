use prism_protocol_sdk::build_claim_tokens_v0_ix;
use prism_protocol_testing::{deterministic_keypair, FixtureStage, TestFixture};
use rust_decimal::prelude::ToPrimitive;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use std::collections::HashMap;

/// Test claim when vault balance < claim amount → SPL Token transfer failure
///
/// **HIGH BUG POTENTIAL**: This test targets the interaction between our calculation logic
/// and SPL Token's transfer validation, which could expose arithmetic or validation bugs.
///
/// **What this tests:**
/// - Token transfer when vault balance is insufficient for claim
/// - Exact boundary conditions (vault balance = claim amount - 1)
/// - SPL Token program error handling vs our validation
/// - State consistency when token transfer fails
///
/// **Why this is critical:**
/// Our claim logic calculates: `total_amount = amount_per_entitlement * entitlements`
/// Then calls: `token::transfer(vault -> claimant, total_amount)`
///
/// The SPL Token program has its own validation that vault balance >= transfer amount.
/// Potential bugs:
/// - Our calculation succeeds but SPL transfer fails
/// - Race condition: vault balance changes between calculation and transfer
/// - Precision issues in large number arithmetic
/// - Edge case: vault balance = 0 exactly
/// - Edge case: vault balance = claim amount - 1 (boundary)
///
/// **Test Strategy:**
/// 1. Set up valid claim with calculated claim amount
/// 2. Use custom funding to create insufficient vault balance
/// 3. Use custom activation to bypass balance validation
/// 4. Attempt claim → should fail with SPL Token error
/// 5. Verify proper error propagation and no state corruption
///
/// **Expected behavior:** SPL Token transfer failure, proper error propagation, no state corruption
#[test]
fn test_claim_vault_balance_insufficient() {
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

    // 3. Calculate expected claim amount and target vault
    let (amount_per_entitlement, vault_address) = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");

        (
            cohort.amount_per_entitlement,
            cohort.vaults[assigned_vault_index as usize].address,
        )
    };

    let expected_claim_amount = amount_per_entitlement
        .checked_mul(rust_decimal::Decimal::from(entitlements))
        .and_then(|d| d.to_u64())
        .expect("Claim amount calculation overflow");
    println!("💰 Expected claim amount: {} tokens", expected_claim_amount);
    println!("🎯 Target vault: {}", vault_address);

    // 4. CRITICAL: Create insufficient vault balance using custom funding
    // Fund vault with exactly (claim_amount - 1) to test precise boundary
    let insufficient_balance = expected_claim_amount - 1;

    let custom_funding = HashMap::from([(vault_address, insufficient_balance)]);

    println!(
        "🔧 Custom funding vault with {} tokens (insufficient for {} claim)",
        insufficient_balance, expected_claim_amount
    );

    test.try_fund_vaults_with_custom_amounts(custom_funding)
        .expect("Custom vault funding should succeed");

    // 5. CRITICAL: Activate vaults with custom expected balance to bypass validation
    // The activate_vault_v0 instruction enforces: vault.amount == expected_balance
    // We need to match our custom funding amount to pass activation
    let custom_expected_balance = HashMap::from([(vault_address, insufficient_balance)]);

    println!(
        "🔧 Activating vault expecting {} tokens (matching our funding)",
        insufficient_balance
    );

    test.try_activate_vaults_with_custom_expected_balance(custom_expected_balance)
        .expect("Custom vault activation should succeed");

    // 6. Continue with normal campaign activation
    test.try_activate_cohorts()
        .expect("Cohort activation should succeed");

    test.try_activate_campaign()
        .expect("Campaign activation should succeed");

    test.advance_slot_by(20); // Past go-live slot

    // 7. Verify our setup: vault should have insufficient balance
    let actual_vault_balance = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    println!("📊 Actual vault balance: {} tokens", actual_vault_balance);
    assert_eq!(
        actual_vault_balance, insufficient_balance,
        "Vault should have our custom insufficient balance"
    );
    assert!(
        actual_vault_balance < expected_claim_amount,
        "Vault balance should be less than claim amount"
    );

    // 8. Airdrop tokens to claimant for transaction fees and ATA creation
    test.airdrop(&claimant_pubkey, 10_000_000); // 0.01 SOL

    println!("📊 State before insufficient balance claim attempt:");
    println!("  Vault balance: {}", actual_vault_balance);
    println!("  Expected claim: {}", expected_claim_amount);
    println!(
        "  Shortfall: {}",
        expected_claim_amount - actual_vault_balance
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

    // 10. Attempt claim with insufficient vault balance → should fail
    println!("🔄 Attempting claim with insufficient vault balance...");

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

    // 11. Verify claim failed with appropriate SPL Token error
    match claim_result {
        Ok(_) => {
            panic!("❌ Claim should have failed due to insufficient vault balance!");
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
            println!("✅ Confirmed SPL Token insufficient balance error");
        }
    }

    // 12. Verify no state corruption occurred
    // The vault balance should be unchanged (no partial transfer)
    let vault_balance_after_failed_claim = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    assert_eq!(
        vault_balance_after_failed_claim, actual_vault_balance,
        "Vault balance should be unchanged after failed claim"
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
        "ClaimReceipt PDA should not exist after failed claim"
    );

    // 14. Verify claimant token account either doesn't exist or has 0 balance
    let claimant_balance = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0);
    assert_eq!(
        claimant_balance, 0,
        "Claimant should have received no tokens"
    );

    println!("✅ Verified no state corruption during insufficient balance failure");

    println!("🎉 Insufficient vault balance test completed successfully!");
    println!("   ❌ Boundary case (claim_amount - 1) correctly blocked claim");
    println!("   🔬 Verified no state corruption during failure");
    println!("   ✅ SPL Token validation working properly");
    println!("   🛠️  Custom funding/activation methods working correctly");

    // 🎓 KEY LEARNINGS FOR DEVELOPERS:
    //
    // 1. CUSTOM TESTING INFRASTRUCTURE: The new try_fund_vaults_with_custom_amounts and
    //    try_activate_vaults_with_custom_expected_balance methods enable precise edge case testing.
    //
    // 2. LAYERED VALIDATION: Our program validates business logic, but SPL Token
    //    validates the actual transfer. Both layers are important.
    //
    // 3. BOUNDARY CONDITIONS: Testing exact boundaries (claim_amount - 1) reveals
    //    precise error handling behavior and confirms SPL Token validation works.
    //
    // 4. ERROR PROPAGATION: SPL Token errors bubble up correctly through our program.
    //    Failed transfers don't create partial state corruption.
    //
    // 5. STATE ATOMICITY: Failed token transfers don't create ClaimReceipt PDAs,
    //    maintaining system consistency.
    //
    // 6. TESTING METHODOLOGY: HashMap-based selective overrides allow testing edge cases
    //    while maintaining normal behavior for non-target components.
}

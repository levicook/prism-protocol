use litesvm::LiteSVM;
use prism_protocol_sdk::{
    build_claim_tokens_v1_ix, CompiledCohortExt as _, CompiledLeafExt as _, CompiledProofExt as _,
};
use prism_protocol_testing::{
    deterministic_keypair, FixtureStage, FixtureState, TestFixture, DEFAULT_BUDGET,
    DEFAULT_MINT_DECIMALS,
};
use rust_decimal::prelude::*;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;

/// Test the full campaign deployment and claiming flow (comprehensive happy path)
///
/// This test demonstrates:
/// - Complete campaign deployment lifecycle (initialize → activate)
/// - Successful token claiming with deterministic claimants
/// - Surgical balance verification (claimant & vault)
/// - Claim receipt PDA creation
/// - Multiple claimants across different cohorts
#[tokio::test]
async fn test_full_campaign_flow_happy_path() {
    let state = FixtureState::new().await;

    let mut test = TestFixture::new(state, LiteSVM::new())
        .await
        .expect("Failed to create test fixture");

    // Step 1-6: Complete deployment lifecycle
    println!("🚀 Deploying campaign through complete lifecycle...");
    test.jump_to(FixtureStage::CampaignActivated).await;

    // Step 7: Wait for go-live slot to pass
    println!("⏰ Advancing past go-live slot...");
    test.advance_slot_by(20); // Ensure we're past go-live

    // Step 8: Test claiming with deterministic claimants
    test_claim_as_early_adopter_1(&mut test).await;
    test_claim_as_investor_2(&mut test).await;
    test_claim_multi_cohort_user(&mut test).await;

    // Test complete vault drainage - precision funding validation
    test_complete_vault_drainage(&mut test).await;

    println!("🎉 Full campaign flow completed successfully!");
}

/// Test claiming tokens as early_adopter_1
/// Based on fixture data: EarlyAdopters cohort, 1 entitlement, 5% share of 1B budget
async fn test_claim_as_early_adopter_1(test: &mut TestFixture) {
    println!("💰 Testing claim as early_adopter_1...");

    // Get claimant keypair
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    // Get mint address and create token account
    let mint = test.state.mint_address();
    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    // Airdrop for fees
    test.airdrop(&claimant_pubkey, 1_000_000_000);

    // Calculate expected tokens based on fixture data
    // EarlyAdopters: 5% of 1B budget = 50M tokens
    // early_adopter_1: 1 entitlement, early_adopter_2: 2 entitlements = 3 total entitlements
    // Expected: (50M / 3) * 1 = 16,666,666,666 token amount (16.666666666 human amount)
    let early_adopters_share = Decimal::from(5); // 5%
    let early_adopters_budget = DEFAULT_BUDGET * (early_adopters_share / Decimal::from(100));
    let total_early_adopters_entitlements = 1 + 2; // early_adopter_1 + early_adopter_2
    let expected_human_per_entitlement =
        early_adopters_budget / Decimal::from(total_early_adopters_entitlements);
    let early_adopter_1_entitlements = 1;
    let expected_human_amount =
        expected_human_per_entitlement * Decimal::from(early_adopter_1_entitlements);

    // Convert human amount to token amount
    let expected_token_amount = (expected_human_amount
        * Decimal::from(10_u64.pow(DEFAULT_MINT_DECIMALS as u32)))
    .floor()
    .to_u64()
    .expect("Expected human amount should convert to valid token amount");

    // Get cohort and vault addresses
    let cohorts = test.state.compiled_cohorts().await;
    let early_adopters_cohort = cohorts
        .iter()
        .find(|c| c.cohort_csv_row_id == 1) // Assuming EarlyAdopters is first
        .expect("EarlyAdopters cohort should exist");

    let cohort_address = early_adopters_cohort.address();

    // Get claimant's leaf data
    let leaf = test
        .state
        .ccdb
        .compiled_leaf_by_cohort_and_claimant(cohort_address, claimant_pubkey)
        .await;

    // Get vault address
    let (vault_address, _) = test
        .state
        .address_finder()
        .find_vault_v0_address(&cohort_address, leaf.vault_index());

    // Record balances BEFORE claim
    let vault_balance_before = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    let claimant_balance_before = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0);

    println!("  📊 Before claim:");
    println!("    Vault balance: {}", vault_balance_before);
    println!("    Claimant balance: {}", claimant_balance_before);
    println!(
        "    Expected claim: {} token amount ({} human amount)",
        expected_token_amount, expected_human_amount
    );

    // Get merkle proof
    let proof = test
        .state
        .ccdb
        .compiled_proofs_by_claimant(claimant_pubkey)
        .await
        .into_iter()
        .find(|p| p.cohort_address() == cohort_address)
        .expect("Should find proof for early_adopter_1 in EarlyAdopters");

    // Build claim instruction using the new API
    let (claim_ix, _, _) = build_claim_tokens_v1_ix(
        test.state.address_finder(),
        claimant_pubkey,
        early_adopters_cohort.merkle_root(),
        proof.merkle_proof_v1(),
        leaf.vault_index(),
        leaf.entitlements(),
    )
    .expect("Failed to build claim instruction");

    // Execute claim
    let claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    test.send_transaction(claim_tx)
        .expect("Claim transaction should succeed");

    // Verify balances AFTER claim
    let vault_balance_after = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    let claimant_balance_after = test
        .get_token_account_balance(&claimant_token_account)
        .expect("Claimant token account should exist after claim");

    println!("  📊 After claim:");
    println!("    Vault balance: {}", vault_balance_after);
    println!("    Claimant balance: {}", claimant_balance_after);

    // ✅ Surgical verification using known expected amounts
    assert_eq!(
        vault_balance_after,
        vault_balance_before - expected_token_amount,
        "Vault balance should decrease by claimed amount"
    );
    assert_eq!(
        claimant_balance_after,
        claimant_balance_before + expected_token_amount,
        "Claimant balance should increase by claimed amount"
    );

    // Verify claim receipt was created
    let (claim_receipt_address, _) = test
        .state
        .address_finder()
        .find_claim_receipt_v0_address(&cohort_address, &claimant_pubkey);

    assert!(
        test.account_exists(&claim_receipt_address),
        "Claim receipt PDA should be created"
    );

    println!(
        "  ✅ early_adopter_1 successfully claimed {} token amount ({} human amount) from EarlyAdopters",
        expected_token_amount, expected_human_amount
    );
}

/// Test claiming tokens as investor_2
/// Based on fixture data: Investors cohort, 2 entitlements, 10% share of 1B budget
async fn test_claim_as_investor_2(test: &mut TestFixture) {
    println!("💰 Testing claim as investor_2...");

    let claimant_keypair = deterministic_keypair("investor_2");
    let claimant_pubkey = claimant_keypair.pubkey();

    let mint = test.state.mint_address();
    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    test.airdrop(&claimant_pubkey, 1_000_000_000);

    // Calculate expected tokens based on fixture data
    // Investors: 10% of 1B budget = 100M tokens
    // investor_1: 1 entitlement, investor_2: 2 entitlements = 3 total entitlements
    // Expected: (100M / 3) * 2 = 66,666,666,666 token amount (66.666666666 human amount)
    let investors_share = Decimal::from(10); // 10%
    let investors_budget = DEFAULT_BUDGET * (investors_share / Decimal::from(100));
    let total_investors_entitlements = 1 + 2; // investor_1 + investor_2
    let expected_human_per_entitlement =
        investors_budget / Decimal::from(total_investors_entitlements);
    let investor_2_entitlements = 2;
    let expected_human_amount =
        expected_human_per_entitlement * Decimal::from(investor_2_entitlements);

    // Convert human amount to token amount
    let expected_token_amount = (expected_human_amount
        * Decimal::from(10_u64.pow(DEFAULT_MINT_DECIMALS as u32)))
    .floor()
    .to_u64()
    .expect("Expected human amount should convert to valid token amount");

    // Get cohort and vault info
    let cohorts = test.state.compiled_cohorts().await;
    let investors_cohort = cohorts
        .iter()
        .find(|c| c.cohort_csv_row_id == 2) // Assuming Investors is second
        .expect("Investors cohort should exist");

    let cohort_address = investors_cohort.address();

    let leaf = test
        .state
        .ccdb
        .compiled_leaf_by_cohort_and_claimant(cohort_address, claimant_pubkey)
        .await;

    let (vault_address, _) = test
        .state
        .address_finder()
        .find_vault_v0_address(&cohort_address, leaf.vault_index());

    // Record balances BEFORE claim
    let vault_balance_before = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");
    let claimant_balance_before = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0);

    println!("  📊 Before claim:");
    println!("    Vault balance: {}", vault_balance_before);
    println!("    Claimant balance: {}", claimant_balance_before);
    println!(
        "    Expected claim: {} token amount ({} human amount)",
        expected_token_amount, expected_human_amount
    );

    // Get merkle proof
    let proof = test
        .state
        .ccdb
        .compiled_proofs_by_claimant(claimant_pubkey)
        .await
        .into_iter()
        .find(|p| p.cohort_address() == cohort_address)
        .expect("Should find proof for investor_2 in Investors");

    let (claim_ix, _, _) = build_claim_tokens_v1_ix(
        test.state.address_finder(),
        claimant_pubkey,
        investors_cohort.merkle_root(),
        proof.merkle_proof_v1(),
        leaf.vault_index(),
        leaf.entitlements(),
    )
    .expect("Failed to build claim instruction");

    let claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    test.send_transaction(claim_tx)
        .expect("Claim transaction should succeed");

    // Verify balances AFTER claim
    let vault_balance_after = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");
    let claimant_balance_after = test
        .get_token_account_balance(&claimant_token_account)
        .expect("Claimant token account should exist after claim");

    println!("  📊 After claim:");
    println!("    Vault balance: {}", vault_balance_after);
    println!("    Claimant balance: {}", claimant_balance_after);

    // ✅ Surgical verification using known expected amounts
    assert_eq!(
        vault_balance_after,
        vault_balance_before - expected_token_amount,
        "Vault balance should decrease by claimed amount"
    );
    assert_eq!(
        claimant_balance_after,
        claimant_balance_before + expected_token_amount,
        "Claimant balance should increase by claimed amount"
    );

    println!(
        "  ✅ investor_2 successfully claimed {} token amount ({} human amount) from Investors",
        expected_token_amount, expected_human_amount
    );
}

/// Test claiming tokens as multi_cohort_user (appears in both PowerUsers AND Team)
/// Based on fixture data:
/// - PowerUsers: 10% share, 5 entitlements for multi_cohort_user (+ others)
/// - Team: 75% share, 10 entitlements for multi_cohort_user (+ others)
async fn test_claim_multi_cohort_user(test: &mut TestFixture) {
    println!("💰 Testing multi-cohort claims as multi_cohort_user...");

    let claimant_keypair = deterministic_keypair("multi_cohort_user");
    let claimant_pubkey = claimant_keypair.pubkey();

    let mint = test.state.mint_address();
    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    test.airdrop(&claimant_pubkey, 1_000_000_000);

    let mut total_claimed = 0u64;
    let initial_balance = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0);

    // Get all cohorts
    let cohorts = test.state.compiled_cohorts().await;

    // Get PowerUsers cohort (10% share)
    let power_users_cohort = cohorts
        .iter()
        .find(|c| c.cohort_csv_row_id == 3) // Assuming PowerUsers is third
        .expect("PowerUsers cohort should exist");

    // Get Team cohort (75% share)
    let team_cohort = cohorts
        .iter()
        .find(|c| c.cohort_csv_row_id == 4) // Assuming Team is fourth
        .expect("Team cohort should exist");

    // PowerUsers expected claim amounts - STATIC VALUES from production budget allocator
    //
    // These values are produced by the budget allocator with its specific precision/rounding logic.
    // PowerUsers cohort: 10% of 1B budget = 100,000,000,000,000,000 lamports total
    // Total entitlements: power_user_1(1) + power_user_2(2) + power_user_3(3) + multi_cohort_user(5) = 11
    // multi_cohort_user gets 5 entitlements out of 11 total
    //
    // DUST NOTE: The budget allocator's precision creates small rounding differences.
    // These static values match what the production code actually produces.
    let power_users_vault_balance_before_expected = 99_999_999_999_999_990u64; // Funded minus 10 lamport dust
    let power_users_claim_amount_expected = 45_454_545_454_545_450u64; // Actual claim amount
    let power_users_vault_balance_after_expected = 54_545_454_545_454_540u64; // Remaining balance (corrected for dust fix)

    // Calculate Team expected claim
    // Team: 75% of 1B = 750M tokens
    // Total entitlements: team_member_1(1) + team_member_2(2) + team_member_3(3) + multi_cohort_user(10) = 16
    // multi_cohort_user gets: (750M / 16) * 10 = 468,750,000,000 lamports
    let team_share = Decimal::from(75);
    let team_budget = DEFAULT_BUDGET * (team_share / Decimal::from(100));
    let total_team_entitlements = 1 + 2 + 3 + 10; // 16 total
    let team_human_per_entitlement = team_budget / Decimal::from(total_team_entitlements);
    let multi_cohort_team_entitlements = 10;
    let team_human_amount =
        team_human_per_entitlement * Decimal::from(multi_cohort_team_entitlements);
    // Convert human amount to token amount
    let team_token_amount = (team_human_amount
        * Decimal::from(10_u64.pow(DEFAULT_MINT_DECIMALS as u32)))
    .floor()
    .to_u64()
    .expect("Expected human amount should convert to valid token amount");

    // Claim from PowerUsers
    println!("  💰 Claiming from PowerUsers cohort...");
    let power_users_leaf = test
        .state
        .ccdb
        .compiled_leaf_by_cohort_and_claimant(power_users_cohort.address(), claimant_pubkey)
        .await;

    // Verify leaf data matches expected values
    assert_eq!(
        power_users_leaf.entitlements(),
        5,
        "multi_cohort_user should have 5 entitlements in PowerUsers"
    );

    let (power_users_vault_address, _) = test.state.address_finder().find_vault_v0_address(
        &power_users_cohort.address(),
        power_users_leaf.vault_index(),
    );

    let power_users_vault_balance_before = test
        .get_token_account_balance(&power_users_vault_address)
        .expect("Should be able to read vault balance");

    // Verify vault balance matches expected initial funding
    assert_eq!(
        power_users_vault_balance_before, power_users_vault_balance_before_expected,
        "PowerUsers vault should be funded with expected amount"
    );

    let power_users_proof = test
        .state
        .ccdb
        .compiled_proofs_by_claimant(claimant_pubkey)
        .await
        .into_iter()
        .find(|p| p.cohort_address() == power_users_cohort.address())
        .expect("Should find proof for multi_cohort_user in PowerUsers");

    let (power_users_claim_ix, _, _) = build_claim_tokens_v1_ix(
        test.state.address_finder(),
        claimant_pubkey,
        power_users_cohort.merkle_root(),
        power_users_proof.merkle_proof_v1(),
        power_users_leaf.vault_index(),
        power_users_leaf.entitlements(),
    )
    .expect("Failed to build claim instruction");

    let power_users_claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[power_users_claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    test.send_transaction(power_users_claim_tx)
        .expect("PowerUsers claim transaction should succeed");

    let power_users_vault_balance_after = test
        .get_token_account_balance(&power_users_vault_address)
        .expect("Should be able to read vault balance");

    // Verify vault balance after claim matches expected value (accounts for budget allocator precision)
    assert_eq!(
        power_users_vault_balance_after, power_users_vault_balance_after_expected,
        "PowerUsers vault balance should match expected value after claim (production precision)"
    );

    // Verify the actual claim amount matches expected
    let actual_claim_amount = power_users_vault_balance_before - power_users_vault_balance_after;
    assert_eq!(
        actual_claim_amount, power_users_claim_amount_expected,
        "Actual claim amount should match production budget allocator value"
    );

    total_claimed += power_users_claim_amount_expected;
    println!(
        "    ✅ Claimed {} token amount from PowerUsers (production precision)",
        power_users_claim_amount_expected
    );

    // Claim from Team
    println!("  💰 Claiming from Team cohort...");
    let team_leaf = test
        .state
        .ccdb
        .compiled_leaf_by_cohort_and_claimant(team_cohort.address(), claimant_pubkey)
        .await;

    let (team_vault_address, _) = test
        .state
        .address_finder()
        .find_vault_v0_address(&team_cohort.address(), team_leaf.vault_index());

    let team_vault_balance_before = test
        .get_token_account_balance(&team_vault_address)
        .expect("Should be able to read vault balance");

    let team_proof = test
        .state
        .ccdb
        .compiled_proofs_by_claimant(claimant_pubkey)
        .await
        .into_iter()
        .find(|p| p.cohort_address() == team_cohort.address())
        .expect("Should find proof for multi_cohort_user in Team");

    let (team_claim_ix, _, _) = build_claim_tokens_v1_ix(
        test.state.address_finder(),
        claimant_pubkey,
        team_cohort.merkle_root(),
        team_proof.merkle_proof_v1(),
        team_leaf.vault_index(),
        team_leaf.entitlements(),
    )
    .expect("Failed to build claim instruction");

    let team_claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[team_claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    test.send_transaction(team_claim_tx)
        .expect("Team claim transaction should succeed");

    let team_vault_balance_after = test
        .get_token_account_balance(&team_vault_address)
        .expect("Should be able to read vault balance");

    assert_eq!(
        team_vault_balance_after,
        team_vault_balance_before - team_token_amount,
        "Team vault balance should decrease by claimed amount"
    );

    total_claimed += team_token_amount;
    println!(
        "    ✅ Claimed {} token amount ({} human amount) from Team",
        team_token_amount, team_human_amount
    );

    // Verify total claimant balance
    let final_balance = test
        .get_token_account_balance(&claimant_token_account)
        .expect("Claimant token account should exist after claims");

    assert_eq!(
        final_balance,
        initial_balance + total_claimed,
        "Claimant balance should increase by total claimed amount"
    );

    println!(
        "  🎉 multi_cohort_user successfully claimed {} total token amount across 2 cohorts",
        total_claimed
    );
}

/// Test complete vault drainage - validate precision funding by draining EarlyAdopters vault
/// This tests our budget allocation precision by claiming ALL remaining entitlements from a vault
/// and verifying it goes to exactly 0 balance (no dust, no leftover tokens)
async fn test_complete_vault_drainage(test: &mut TestFixture) {
    println!("🏁 Testing complete vault drainage (precision funding validation)...");

    // We already claimed early_adopter_1 (1 entitlement)
    // Now claim early_adopter_2 (2 entitlements) to completely drain the EarlyAdopters vault
    let claimant_keypair = deterministic_keypair("early_adopter_2");
    let claimant_pubkey = claimant_keypair.pubkey();

    let mint = test.state.mint_address();
    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    test.airdrop(&claimant_pubkey, 1_000_000_000);

    // EarlyAdopters vault drainage - STATIC VALUES from production system
    //
    // After early_adopter_1 claimed 16,666,666,666,666,666 lamports, the vault should contain
    // exactly what early_adopter_2 is entitled to claim (2 entitlements vs early_adopter_1's 1).
    //
    // DUST FIX: Vaults are now funded with budget minus dust to enable perfect drainage.
    // - Vault funded with: 49,999,999,999,999,998 lamports (budget - 2 lamport dust)
    // - early_adopter_1 claimed: 16,666,666,666,666,666 lamports
    // - Remaining for early_adopter_2: 33,333,333,333,333,332 lamports
    // - Perfect drainage: 0 lamports remain after complete claim
    let vault_balance_before_drainage_expected = 33_333_333_333_333_332u64; // Remaining after early_adopter_1
    let early_adopter_2_claim_amount_expected = 33_333_333_333_333_332u64; // All remaining tokens
    let vault_balance_after_drainage_expected = 0u64; // Perfect drainage!

    // Get EarlyAdopters cohort
    let cohorts = test.state.compiled_cohorts().await;
    let early_adopters_cohort = cohorts
        .iter()
        .find(|c| c.cohort_csv_row_id == 1) // EarlyAdopters is first
        .expect("EarlyAdopters cohort should exist");

    let cohort_address = early_adopters_cohort.address();

    // Get claimant's leaf data
    let leaf = test
        .state
        .ccdb
        .compiled_leaf_by_cohort_and_claimant(cohort_address, claimant_pubkey)
        .await;

    // Get vault address (same vault as early_adopter_1 used)
    let (vault_address, _) = test
        .state
        .address_finder()
        .find_vault_v0_address(&cohort_address, leaf.vault_index());

    // Record balances BEFORE final claim
    let vault_balance_before = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    let claimant_balance_before = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0);

    println!("  📊 Before final claim (vault drainage):");
    println!("    Vault balance: {} token amount", vault_balance_before);
    println!(
        "    Claimant balance: {} token amount",
        claimant_balance_before
    );
    println!(
        "    Expected claim: {} token amount (production precision)",
        early_adopter_2_claim_amount_expected
    );
    println!("    Expected vault balance after: 0 token amount (perfect drainage)");

    // Verify vault contains exactly what production budget allocator determined
    assert_eq!(
        vault_balance_before, vault_balance_before_drainage_expected,
        "Vault should contain expected remaining balance after early_adopter_1 claim (production precision)"
    );

    // Get merkle proof
    let proof = test
        .state
        .ccdb
        .compiled_proofs_by_claimant(claimant_pubkey)
        .await
        .into_iter()
        .find(|p| p.cohort_address() == cohort_address)
        .expect("Should find proof for early_adopter_2 in EarlyAdopters");

    // Build and execute claim instruction
    let (claim_ix, _, _) = build_claim_tokens_v1_ix(
        test.state.address_finder(),
        claimant_pubkey,
        early_adopters_cohort.merkle_root(),
        proof.merkle_proof_v1(),
        leaf.vault_index(),
        leaf.entitlements(),
    )
    .expect("Failed to build claim instruction");

    let claim_tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[claim_ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    test.send_transaction(claim_tx)
        .expect("Final claim transaction should succeed");

    // Verify balances AFTER complete drainage
    let vault_balance_after = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");

    let claimant_balance_after = test
        .get_token_account_balance(&claimant_token_account)
        .expect("Claimant token account should exist after claim");

    println!("  📊 After complete vault drainage:");
    println!("    Vault balance: {} token amount", vault_balance_after);
    println!(
        "    Claimant balance: {} token amount",
        claimant_balance_after
    );

    // 🎯 THE PRECISION TEST: Vault should be EXACTLY 0 (perfect drainage)
    assert_eq!(
        vault_balance_after, vault_balance_after_drainage_expected,
        "🔍 PRECISION TEST: Vault should be perfectly drained after claiming all entitlements"
    );

    // Verify claimant received correct amount (production precision)
    assert_eq!(
        claimant_balance_after,
        claimant_balance_before + early_adopter_2_claim_amount_expected,
        "Claimant balance should increase by expected claim amount (production precision)"
    );

    // Verify claim receipt was created
    let (claim_receipt_address, _) = test
        .state
        .address_finder()
        .find_claim_receipt_v0_address(&cohort_address, &claimant_pubkey);

    assert!(
        test.account_exists(&claim_receipt_address),
        "Claim receipt PDA should be created"
    );

    println!(
        "  ✅ early_adopter_2 successfully claimed {} token amount from EarlyAdopters (production precision)",
        early_adopter_2_claim_amount_expected
    );
    println!("  🎯 PRECISION TEST PASSED: Vault perfectly drained to 0 token amount!");
    println!("     Mathematical dust kept by admin, not over-funded to vault! 🎉");
}

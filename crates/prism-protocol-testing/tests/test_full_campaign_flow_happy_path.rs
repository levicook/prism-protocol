use prism_protocol_sdk::{build_claim_tokens_v0_ix, ClaimLeaf, CompiledCohort};
use prism_protocol_testing::{deterministic_keypair, FixtureStage, TestFixture};
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
#[test]
fn test_full_campaign_flow_happy_path() {
    let mut test = TestFixture::default();

    // Step 1-6: Complete deployment lifecycle
    println!("🚀 Deploying campaign through complete lifecycle...");
    test.jump_to(FixtureStage::CampaignActivated);

    // Step 7: Wait for go-live slot to pass
    println!("⏰ Advancing past go-live slot...");
    test.advance_slot_by(20); // Ensure we're past go-live

    // Step 8: Test claiming with deterministic claimants
    test_claim_as_early_adopter_1(&mut test);
    test_claim_as_investor_2(&mut test);
    test_claim_multi_cohort_user(&mut test);

    println!("🎉 Full campaign flow completed successfully!");
}

/// Test claiming tokens as early_adopter_1
fn test_claim_as_early_adopter_1(test: &mut TestFixture) {
    println!("💰 Testing claim as early_adopter_1...");

    // Get claimant keypair
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    // 🎯 Extract all needed data before any mutable borrows
    let (
        vault_address,
        expected_tokens_u64,
        admin,
        mint,
        address_finder,
        fingerprint,
        merkle_root,
        assigned_vault_index,
        entitlements,
        cohort_address,
    ) = {
        let (cohort, leaf) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");

        let vault = cohort
            .find_claimant_vault(&claimant_pubkey)
            .expect("Claimant should have assigned vault");

        let expected_tokens_u64 = cohort
            .expected_claim_amount_u64(&claimant_pubkey)
            .expect("Should be able to calculate expected claim");

        (
            vault.address,
            expected_tokens_u64,
            test.state.compiled_campaign.admin,
            test.state.compiled_campaign.mint,
            test.state.address_finder.clone(),
            test.state.compiled_campaign.fingerprint,
            cohort.merkle_root,
            leaf.assigned_vault_index,
            leaf.entitlements,
            cohort.address,
        )
    };

    // Get claimant's token account
    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    // Create token account if needed and airdrop for fees
    test.airdrop(&claimant_pubkey, 1_000_000_000);

    // Record balances BEFORE claim
    let vault_balance_before = test
        .get_token_account_balance(&vault_address)
        .expect("Should be able to read vault balance");
    let claimant_balance_before = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0); // May not exist yet

    println!("  📊 Before claim:");
    println!("    Vault balance: {}", vault_balance_before);
    println!("    Claimant balance: {}", claimant_balance_before);
    println!("    Expected claim: {}", expected_tokens_u64);

    // 🎯 Generate proof using extracted data
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

    // Build claim instruction
    let (claim_ix, _, _) = build_claim_tokens_v0_ix(
        &address_finder,
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

    // ✅ Surgical verification
    assert_eq!(
        vault_balance_after,
        vault_balance_before - expected_tokens_u64,
        "Vault balance should decrease by claimed amount"
    );
    assert_eq!(
        claimant_balance_after,
        claimant_balance_before + expected_tokens_u64,
        "Claimant balance should increase by claimed amount"
    );

    // Verify claim receipt was created
    let (claim_receipt_address, _) =
        address_finder.find_claim_receipt_v0_address(&cohort_address, &claimant_pubkey);

    // Check that claim receipt account exists
    assert!(
        test.account_exists(&claim_receipt_address),
        "Claim receipt PDA should be created"
    );

    println!(
        "  ✅ early_adopter_1 successfully claimed {} tokens from EarlyAdopters",
        expected_tokens_u64
    );
}

/// Test claiming tokens as investor_2  
fn test_claim_as_investor_2(test: &mut TestFixture) {
    println!("💰 Testing claim as investor_2...");

    // Get claimant keypair
    let claimant_keypair = deterministic_keypair("investor_2");
    let claimant_pubkey = claimant_keypair.pubkey();

    // 🎯 Extract all needed data before any mutable borrows
    let (
        vault_address,
        expected_tokens_u64,
        admin,
        mint,
        address_finder,
        fingerprint,
        merkle_root,
        assigned_vault_index,
        entitlements,
    ) = {
        let (cohort, leaf) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "Investors")
            .expect("investor_2 should be in Investors cohort");

        let vault = cohort
            .find_claimant_vault(&claimant_pubkey)
            .expect("Claimant should have assigned vault");

        let expected_tokens_u64 = cohort
            .expected_claim_amount_u64(&claimant_pubkey)
            .expect("Should be able to calculate expected claim");

        (
            vault.address,
            expected_tokens_u64,
            test.state.compiled_campaign.admin,
            test.state.compiled_campaign.mint,
            test.state.address_finder.clone(),
            test.state.compiled_campaign.fingerprint,
            cohort.merkle_root,
            leaf.assigned_vault_index,
            leaf.entitlements,
        )
    };

    // Same verification flow as early_adopter_1 but with different claimant
    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    test.airdrop(&claimant_pubkey, 1_000_000_000);

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
    println!("    Expected claim: {}", expected_tokens_u64);

    // Generate merkle proof using extracted data
    let proof = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "Investors")
            .expect("investor_2 should be in Investors cohort");
        cohort
            .proof_for_claimant(&claimant_pubkey)
            .expect("Should be able to generate proof")
    };

    let (claim_ix, _, _) = build_claim_tokens_v0_ix(
        &address_finder,
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

    // ✅ Surgical verification
    assert_eq!(
        vault_balance_after,
        vault_balance_before - expected_tokens_u64,
        "Vault balance should decrease by claimed amount"
    );
    assert_eq!(
        claimant_balance_after,
        claimant_balance_before + expected_tokens_u64,
        "Claimant balance should increase by claimed amount"
    );

    println!(
        "  ✅ investor_2 successfully claimed {} tokens from Investors",
        expected_tokens_u64
    );
}

/// Test claiming tokens as multi_cohort_user (appears in both PowerUsers AND Team)
fn test_claim_multi_cohort_user(test: &mut TestFixture) {
    println!("💰 Testing multi-cohort claims as multi_cohort_user...");

    let claimant_keypair = deterministic_keypair("multi_cohort_user");
    let claimant_pubkey = claimant_keypair.pubkey();

    let cohort_data: Vec<(CompiledCohort, ClaimLeaf)> = test
        .state
        .compiled_campaign
        .find_claimant_in_all_cohorts(&claimant_pubkey);

    let admin = test.state.compiled_campaign.admin;
    let mint = test.state.compiled_campaign.mint;
    let address_finder = test.state.address_finder.clone();
    let fingerprint = test.state.compiled_campaign.fingerprint;

    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    test.airdrop(&claimant_pubkey, 1_000_000_000);

    let mut total_claimed = 0u64;
    let initial_balance = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0);

    // Claim from each cohort using extracted data
    for (cohort, leaf) in cohort_data {
        println!("  💰 Claiming from {} cohort...", cohort.name);

        // Record vault balance before
        let vault_balance_before = test
            .get_token_account_balance(&cohort.vaults[leaf.assigned_vault_index as usize].address)
            .expect("Should be able to read vault balance");

        // Generate proof for this specific cohort
        let proof = cohort
            .proof_for_claimant(&claimant_pubkey)
            .expect("Should be able to generate proof");

        let (claim_ix, _, _) = build_claim_tokens_v0_ix(
            &address_finder,
            admin,
            claimant_pubkey,
            mint,
            claimant_token_account,
            fingerprint,
            cohort.merkle_root,
            proof,
            leaf.assigned_vault_index,
            leaf.entitlements,
        )
        .expect("Failed to build claim instruction");

        let claim_tx = Transaction::new(
            &[&claimant_keypair],
            Message::new(&[claim_ix], Some(&claimant_pubkey)),
            test.latest_blockhash(),
        );

        test.send_transaction(claim_tx)
            .expect("Claim transaction should succeed");

        // Verify vault balance decreased
        let vault_balance_after = test
            .get_token_account_balance(&cohort.vaults[leaf.assigned_vault_index as usize].address)
            .expect("Should be able to read vault balance");

        let expected_tokens_u64 = cohort
            .expected_claim_amount_u64(&claimant_pubkey)
            .expect("Should be able to calculate expected claim");

        assert_eq!(
            vault_balance_after,
            vault_balance_before - expected_tokens_u64,
            "Vault balance should decrease by claimed amount in {} cohort",
            cohort.name
        );

        total_claimed += expected_tokens_u64;
        println!(
            "    ✅ Claimed {} tokens from {}",
            expected_tokens_u64, cohort.name
        );
    }

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
        "  🎉 multi_cohort_user successfully claimed {} total tokens across 2 cohorts",
        total_claimed
    );
}

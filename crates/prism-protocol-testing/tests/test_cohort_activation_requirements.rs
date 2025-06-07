use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};
use prism_protocol_sdk::CompiledCohortExt;
use solana_message::Message;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_transaction::Transaction;

/// Test cohort activation requirements → NotAllVaultsActivated
///
/// This test validates that cohorts cannot be activated until ALL expected vaults are activated:
/// - Sets up cohort with multiple expected vaults (3 in this test)
/// - Attempts cohort activation with 0/3 vaults activated → should fail
/// - Activates vaults incrementally (1/3, then 2/3, then 3/3)
/// - Attempts cohort activation after each vault activation
/// - Verifies only the final attempt (3/3 vaults) succeeds
/// - Validates proper state transitions and counter updates
/// - Includes bonus test for single-vault cohort edge case
///
/// **Background**: Cohort activation is a critical milestone that can only happen when
/// ALL vault funding has been verified. This prevents partial cohort activation that
/// could lead to under-funded claim scenarios.
///
/// **Real-world scenario**: Campaign administrators must ensure all vault funding
/// transfers complete before attempting cohort activation. The protocol enforces
/// this ordering to maintain funding guarantees.
///
/// **Technical note**: This test uses compute budget instructions to make retry
/// transactions unique, preventing Solana's duplicate transaction detection from
/// blocking legitimate test cases with `AlreadyProcessed` errors.
///
/// **Test flow**:
/// 1. Set up campaign with 3-vault cohort configuration
/// 2. Advance to VaultsFunded stage (vaults exist but not activated)
/// 3. Verify initial state: all vaults funded, none activated
/// 4. Test cohort activation with 0/3 vaults → expect NotAllVaultsActivated
/// 5. Activate vault 0, test cohort activation with 1/3 vaults → expect NotAllVaultsActivated
/// 6. Activate vault 1, test cohort activation with 2/3 vaults → expect NotAllVaultsActivated  
/// 7. Activate vault 2, test cohort activation with 3/3 vaults → expect SUCCESS
/// 8. Verify final state: cohort activated, counters updated
/// 9. Bonus: Test single-vault cohort (should activate immediately after 1 vault)
#[tokio::test]
async fn test_cohort_activation_requirements() {
    // STEP 1: Set up test fixture with 3-vault cohort configuration
    let fixture_state = FixtureState::with_vault_count(3).await; // 3 vaults per cohort
    let svm = LiteSVM::new();
    let mut test = TestFixture::new(fixture_state, svm).await.unwrap();

    // STEP 2: Jump to VaultsFunded stage (vaults exist and funded, but not yet activated)
    test.jump_to(FixtureStage::VaultsFunded).await;

    // STEP 3: Get the cohort and verify initial configuration
    let cohorts = test.state.compiled_cohorts().await;
    let cohort = &cohorts[0];
    
    println!("🏗️  Testing cohort with {} expected vaults", cohort.vault_count());
    assert_eq!(cohort.vault_count(), 3, "Test expects 3 vaults");

    // Verify initial state: all 3 vaults exist and are funded, but none activated
    println!("📋 Verifying initial state: vaults funded but not activated...");
    for i in 0..cohort.vault_count() {
        let (vault_address, _) = test.state.address_finder().find_vault_v0_address(&cohort.address(), i);
        let vault_balance = test.get_token_account_balance(&vault_address).unwrap_or(0);
        println!("  Vault {}: funded = {}, balance = {}", i, vault_balance > 0, vault_balance);
    }

    // STEP 4: Test cohort activation with 0/3 vaults activated → should fail
    println!("\n❌ STEP 4: Attempting cohort activation with 0/3 vaults activated...");
    let (cohort_activation_ix, _, _) = prism_protocol_sdk::build_activate_cohort_v0_ix(
        test.state.address_finder(),
        cohort.merkle_root(),
    ).expect("Should build cohort activation instruction");
    
    let no_vaults_result = test.send_transaction(Transaction::new(
        &[&test.state.admin_keypair()],
        Message::new(&[cohort_activation_ix], Some(&test.state.admin_address())),
        test.latest_blockhash(),
    ));
    
    demand_prism_error(
        no_vaults_result,
        PrismError::NotAllVaultsActivated as u32,
        "NotAllVaultsActivated",
    );

    // STEP 5: Activate first vault, then test cohort activation with 1/3 vaults
    println!("\n🔧 STEP 5a: Activating vault 1/3...");
    let (vault_0_address, _) = test.state.address_finder().find_vault_v0_address(&cohort.address(), 0);
    let vault_0_balance = test.get_token_account_balance(&vault_0_address).expect("Should get vault 0 balance");
    
    let (activate_vault_0_ix, _, _) = prism_protocol_sdk::build_activate_vault_v0_ix(
        test.state.address_finder(),
        cohort.merkle_root(),
        0,
        vault_0_balance,
    ).expect("Should build vault 0 activation instruction");
    
    test.send_transaction(Transaction::new(
        &[&test.state.admin_keypair()],
        Message::new(&[activate_vault_0_ix], Some(&test.state.admin_address())),
        test.latest_blockhash(),
    )).expect("Should be able to activate first vault");

    println!("❌ STEP 5b: Attempting cohort activation with 1/3 vaults activated...");
    let (cohort_activation_ix_2, _, _) = prism_protocol_sdk::build_activate_cohort_v0_ix(
        test.state.address_finder(),
        cohort.merkle_root(),
    ).expect("Should build cohort activation instruction");
    
    // Add compute budget to make transaction unique (prevents AlreadyProcessed error)
    let compute_budget_ix_1 = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
    
    let partial_result = test.send_transaction(Transaction::new(
        &[&test.state.admin_keypair()],
        Message::new(&[compute_budget_ix_1, cohort_activation_ix_2], Some(&test.state.admin_address())),
        test.latest_blockhash(),
    ));
    
    demand_prism_error(
        partial_result,
        PrismError::NotAllVaultsActivated as u32,
        "NotAllVaultsActivated",
    );

    // STEP 6: Activate second vault, then test cohort activation with 2/3 vaults
    println!("\n🔧 STEP 6a: Activating vault 2/3...");
    let (vault_1_address, _) = test.state.address_finder().find_vault_v0_address(&cohort.address(), 1);
    let vault_1_balance = test.get_token_account_balance(&vault_1_address).expect("Should get vault 1 balance");
    
    let (activate_vault_1_ix, _, _) = prism_protocol_sdk::build_activate_vault_v0_ix(
        test.state.address_finder(),
        cohort.merkle_root(),
        1,
        vault_1_balance,
    ).expect("Should build vault 1 activation instruction");
    
    test.send_transaction(Transaction::new(
        &[&test.state.admin_keypair()],
        Message::new(&[activate_vault_1_ix], Some(&test.state.admin_address())),
        test.latest_blockhash(),
    )).expect("Should be able to activate second vault");

    println!("❌ STEP 6b: Attempting cohort activation with 2/3 vaults activated...");
    let (cohort_activation_ix_3, _, _) = prism_protocol_sdk::build_activate_cohort_v0_ix(
        test.state.address_finder(),
        cohort.merkle_root(),
    ).expect("Should build cohort activation instruction");
    
    // Add different compute budget to make transaction unique (prevents AlreadyProcessed)
    let compute_budget_ix_2 = ComputeBudgetInstruction::set_compute_unit_limit(250_000);
    
    let still_partial_result = test.send_transaction(Transaction::new(
        &[&test.state.admin_keypair()],
        Message::new(&[compute_budget_ix_2, cohort_activation_ix_3], Some(&test.state.admin_address())),
        test.latest_blockhash(),
    ));
    
    demand_prism_error(
        still_partial_result,
        PrismError::NotAllVaultsActivated as u32,
        "NotAllVaultsActivated",
    );

    // STEP 7: Activate third vault, then test cohort activation with 3/3 vaults → SUCCESS!
    println!("\n🔧 STEP 7a: Activating vault 3/3...");
    let (vault_2_address, _) = test.state.address_finder().find_vault_v0_address(&cohort.address(), 2);
    let vault_2_balance = test.get_token_account_balance(&vault_2_address).expect("Should get vault 2 balance");
    
    let (activate_vault_2_ix, _, _) = prism_protocol_sdk::build_activate_vault_v0_ix(
        test.state.address_finder(),
        cohort.merkle_root(),
        2,
        vault_2_balance,
    ).expect("Should build vault 2 activation instruction");
    
    test.send_transaction(Transaction::new(
        &[&test.state.admin_keypair()],
        Message::new(&[activate_vault_2_ix], Some(&test.state.admin_address())),
        test.latest_blockhash(),
    )).expect("Should be able to activate third vault");

    println!("\n✅ STEP 7b: Attempting cohort activation with 3/3 vaults activated...");
    let (final_cohort_activation_ix, _, _) = prism_protocol_sdk::build_activate_cohort_v0_ix(
        test.state.address_finder(),
        cohort.merkle_root(),
    ).expect("Should build final cohort activation instruction");
    
    // Add different compute budget to make this transaction unique (prevents AlreadyProcessed)
    let compute_budget_ix_3 = ComputeBudgetInstruction::set_compute_unit_limit(300_000);
    
    test.send_transaction(Transaction::new(
        &[&test.state.admin_keypair()],
        Message::new(&[compute_budget_ix_3, final_cohort_activation_ix], Some(&test.state.admin_address())),
        test.latest_blockhash(),
    )).expect("Cohort activation should succeed with all vaults activated");

    // STEP 8: Verify final state - cohort activated, counters updated
    let cohort_account = test.fetch_cohort_account(&cohort.address()).expect("Should fetch cohort account");
    assert_eq!(cohort_account.activated_vault_count, 3, "All 3 vaults should be activated");
    assert_eq!(cohort_account.expected_vault_count, 3, "Expected count should be 3");

    let campaign_account = test.fetch_campaign_account().expect("Should fetch campaign account");
    assert_eq!(
        campaign_account.activated_cohort_count, 1,
        "Campaign should show 1 activated cohort"
    );

    println!("✅ STEP 8: Main test completed successfully!");
    println!("   🔒 Partial vault activation correctly blocked cohort activation");
    println!("   ✅ Full vault activation enabled successful cohort activation");
    println!("   📊 Campaign counters updated correctly");

    // STEP 9: BONUS - Test single-vault cohort edge case (should activate immediately)
    println!("\n🎯 STEP 9: Bonus - Testing single-vault cohort edge case...");
    
    // Create separate test fixture with single vault configuration
    let single_vault_fixture = FixtureState::with_vault_count(1).await;
    let svm2 = LiteSVM::new();
    let mut single_test = TestFixture::new(single_vault_fixture, svm2).await.unwrap();
    
    single_test.jump_to(FixtureStage::VaultsFunded).await;
    
    let single_cohorts = single_test.state.compiled_cohorts().await;
    let single_cohort = &single_cohorts[0];
    
    assert_eq!(single_cohort.vault_count(), 1, "Single test expects 1 vault");
    
    // Activate the single vault (1/1)
    let (single_vault_address, _) = single_test.state.address_finder().find_vault_v0_address(&single_cohort.address(), 0);
    let single_vault_balance = single_test.get_token_account_balance(&single_vault_address).expect("Should get single vault balance");
    
    let (activate_single_vault_ix, _, _) = prism_protocol_sdk::build_activate_vault_v0_ix(
        single_test.state.address_finder(),
        single_cohort.merkle_root(),
        0,
        single_vault_balance,
    ).expect("Should build single vault activation instruction");
    
    single_test.send_transaction(Transaction::new(
        &[&single_test.state.admin_keypair()],
        Message::new(&[activate_single_vault_ix], Some(&single_test.state.admin_address())),
        single_test.latest_blockhash(),
    )).expect("Should activate single vault");
    
    // Cohort activation should succeed immediately (1/1 vaults activated)
    let (activate_single_cohort_ix, _, _) = prism_protocol_sdk::build_activate_cohort_v0_ix(
        single_test.state.address_finder(),
        single_cohort.merkle_root(),
    ).expect("Should build single cohort activation instruction");
    
    single_test.send_transaction(Transaction::new(
        &[&single_test.state.admin_keypair()],
        Message::new(&[activate_single_cohort_ix], Some(&single_test.state.admin_address())),
        single_test.latest_blockhash(),
    )).expect("Single-vault cohort should activate successfully");
        
    println!("✅ STEP 9: Single-vault cohort activation works correctly");
}

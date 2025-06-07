use litesvm::LiteSVM;
use prism_protocol_sdk::CompiledCohortExt;
use prism_protocol_testing::{FixtureStage, FixtureState, TestFixture};

/// Test vault counter arithmetic at u8 boundary conditions
///
/// **BACKGROUND**: This test validates that vault counter arithmetic works correctly at
/// u8 boundary conditions, especially with consistent hashing collisions that create
/// zero-entitlement vaults.
///
/// **CRITICAL BUG PREVENTED**: Previously, zero-entitlement vaults (caused by consistent
/// hashing collisions) would fail compilation with "Vault with zero entitlements should
/// not be created". This test ensures our fix allows empty vaults gracefully.
///
/// **What this tests:**
/// - Normal vault counter increments work correctly (0 → 10 vaults)
/// - Maximum capacity counter arithmetic (0 → 255 vaults, u8::MAX boundary)
/// - Complete vault lifecycle with zero-entitlement vaults from hash collisions
/// - Smart contract counter validation: `checked_add(1).ok_or(ErrorCode::NumericOverflow)`
///
/// **Why u8::MAX (255) matters:**
/// Vault indices are u8, so testing 255 vaults exercises the maximum possible vault count
/// and stresses the counter increment logic used throughout the vault lifecycle.
///
/// **Hash collision insight:**
/// With 255 vaults and 1 claimant each, consistent hashing often creates collisions,
/// leaving some vaults empty (0 entitlements). This test proves our system handles
/// these edge cases gracefully rather than failing compilation.
#[tokio::test]
async fn test_vault_counter_u8_boundary_conditions() {
    // === TEST 1: Normal operation baseline ===
    // Verify that basic vault counter arithmetic works correctly with a moderate vault count.
    // This establishes that our counter increment logic works in normal scenarios.
    println!("🧪 Testing normal vault initialization with counter increments...");
    let state = FixtureState::with_vault_count(10).await; // Moderate count for baseline testing
    let mut test = TestFixture::new(state, LiteSVM::new()).await.unwrap();

    // Execute vault initialization - this triggers counter increments in the smart contract
    test.jump_to(FixtureStage::VaultsInitialized).await;

    // Verify the smart contract's counter tracking is accurate
    let cohorts = test.state.compiled_cohorts().await;
    assert_eq!(cohorts.len(), 1, "Should have exactly 1 cohort");

    let cohort_account = test.fetch_cohort_account(&cohorts[0].address()).unwrap();
    assert_eq!(
        cohort_account.initialized_vault_count, 10,
        "Smart contract should track 10 vault initializations"
    );
    assert_eq!(
        cohort_account.expected_vault_count, 10,
        "Expected count should match compiled campaign design"
    );

    println!("✅ Normal vault counter arithmetic works correctly");

    // === TEST 2: u8 boundary stress test ===
    // Test counter arithmetic at u8::MAX capacity (255 vaults). This stresses the
    // checked_add(1) operations and tests for potential overflow bugs.
    // NOTE: With 255 claimants and 1 per vault, consistent hashing creates collisions,
    // resulting in some vaults having 0 entitlements. This tests our collision fix.
    println!("🧪 Testing u8::MAX vault count boundary (255 vaults)...");
    let max_state = FixtureState::with_vault_count(255).await;
    let mut max_test = TestFixture::new(max_state, LiteSVM::new()).await.unwrap();

    // Execute maximum vault initialization - tests counter arithmetic under stress
    max_test.jump_to(FixtureStage::VaultsInitialized).await;

    // Verify that all 255 vault initializations were tracked correctly
    let max_cohorts = max_test.state.compiled_cohorts().await;
    let max_cohort_account = max_test
        .fetch_cohort_account(&max_cohorts[0].address())
        .unwrap();
    assert_eq!(
        max_cohort_account.initialized_vault_count, 255,
        "Smart contract should successfully track 255 vault initializations at u8::MAX"
    );
    assert_eq!(
        max_cohort_account.expected_vault_count, 255,
        "Expected count should match maximum u8 capacity"
    );

    println!("✅ u8::MAX boundary handled without arithmetic overflow");

    // === TEST 3: Complete lifecycle validation ===
    // Continue with the same 255-vault test fixture and execute the full lifecycle.
    // This ensures that vault activation counters also work correctly at maximum capacity,
    // and that zero-entitlement vaults (from hash collisions) don't break activation.
    println!("🧪 Testing complete vault lifecycle at maximum capacity...");
    max_test.jump_to(FixtureStage::VaultsActivated).await;

    // Verify that activation counter arithmetic also works at u8::MAX
    let final_cohort_account = max_test
        .fetch_cohort_account(&max_cohorts[0].address())
        .unwrap();
    assert_eq!(
        final_cohort_account.activated_vault_count, 255,
        "Smart contract should successfully track 255 vault activations at u8::MAX"
    );

    println!("✅ Complete vault lifecycle succeeds at u8::MAX capacity");

    println!("🎉 U8 BOUNDARY CONDITION TESTING COMPLETE:");
    println!("  ✅ Normal operation (10 vaults): Counter arithmetic works correctly");
    println!("  ✅ Maximum capacity (255 vaults): u8::MAX boundary handled without overflow");
    println!("  ✅ Complete lifecycle: Both init + activation counters work at maximum");
    println!("  ✅ Hash collision resilience: Zero-entitlement vaults handled gracefully");
    println!("🔍 Smart contract u8 counter validation complete!");
}

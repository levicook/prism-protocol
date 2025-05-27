#![cfg(all(feature = "testing"))]

use {
    anchor_lang::{prelude::AccountDeserialize, Space as _},
    mollusk_svm::{program::keyed_account_for_system_program, result::Check},
    prism_protocol::{
        self,
        sdk::{
            address_finders::find_cohort_v0_address,
            instruction_builders::build_initialize_cohort_ix,
        },
        state::CohortV0,
        test_utils::{
            generate_test_merkle_root, generate_test_vaults, TestFixture,
            TEST_AMOUNT_PER_ENTITLEMENT,
        },
        ID as PRISM_PROGRAM_ID,
    },
    solana_sdk::{account::Account as SolanaAccount, system_program::ID as SYSTEM_PROGRAM_ID},
};

#[test]
fn test_initialize_cohort_success() {
    // 1. Setup test fixture and initialize campaign
    let mut fixture = TestFixture::new();
    let campaign_result = fixture.initialize_campaign();

    // 2. Prepare cohort data
    let merkle_root = generate_test_merkle_root();
    let amount_per_entitlement = TEST_AMOUNT_PER_ENTITLEMENT;
    let vaults = generate_test_vaults(2); // Test with 2 vaults

    // 3. Derive cohort address
    let (cohort_address, cohort_bump) = find_cohort_v0_address(
        &campaign_result.address, //
        &merkle_root,
    );

    // 4. Build cohort initialization instruction
    let (initialize_cohort_ix, _, _) = build_initialize_cohort_ix(
        fixture.admin_address,
        campaign_result.address,
        fixture.test_fingerprint,
        cohort_address,
        merkle_root,
        amount_per_entitlement,
        vaults.clone(),
    )
    .expect("Failed to build initialize_cohort instruction");

    println!(
        "Initializing cohort: {} (bump: {}, vaults: {})",
        cohort_address,
        cohort_bump,
        vaults.len()
    );

    // 5. Execute cohort initialization
    let result = fixture.mollusk.process_and_validate_instruction(
        &initialize_cohort_ix,
        &[
            keyed_account_for_system_program(),
            (fixture.admin_address, campaign_result.admin_account),
            (campaign_result.address, campaign_result.campaign_account),
            (cohort_address, SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID)),
        ],
        &[
            Check::success(), //
        ],
    );

    println!(
        "Cohort initialized - CU consumed: {}, execution time: {}",
        result.compute_units_consumed, result.execution_time
    );

    // 6. Validate cohort state
    let cohort_account = result
        .get_account(&cohort_address)
        .expect("Cohort account not found");

    assert_eq!(
        cohort_account.owner, PRISM_PROGRAM_ID,
        "owner mismatch: expected: {:?}, actual: {:?}",
        PRISM_PROGRAM_ID, cohort_account.owner
    );

    assert_eq!(
        cohort_account.data.len(),
        CohortV0::INIT_SPACE + 8,
        "account size mismatch: expected: {}, actual: {}",
        CohortV0::INIT_SPACE + 8,
        cohort_account.data.len()
    );

    let cohort_state = CohortV0::try_deserialize(&mut cohort_account.data.as_slice())
        .expect("Failed to deserialize Cohort state");

    // Validate cohort fields
    assert_eq!(
        cohort_state.campaign, campaign_result.address,
        "Campaign address mismatch: expected: {:?}, actual: {:?}",
        campaign_result.address, cohort_state.campaign
    );

    assert_eq!(
        cohort_state.merkle_root, merkle_root,
        "Merkle root mismatch: expected: {:?}, actual: {:?}",
        merkle_root, cohort_state.merkle_root
    );

    assert_eq!(
        cohort_state.amount_per_entitlement, amount_per_entitlement,
        "Amount per entitlement mismatch: expected: {}, actual: {}",
        amount_per_entitlement, cohort_state.amount_per_entitlement
    );

    assert_eq!(
        cohort_state.vaults, vaults,
        "Vaults mismatch: expected: {:?}, actual: {:?}",
        vaults, cohort_state.vaults
    );

    assert_eq!(
        cohort_state.bump, cohort_bump,
        "bump mismatch: expected: {}, actual: {}",
        cohort_bump, cohort_state.bump
    );

    println!("✅ Cohort state validation passed");
}

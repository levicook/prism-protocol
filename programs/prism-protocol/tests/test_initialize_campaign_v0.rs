#![cfg(feature = "test-sbf")]

use {
    anchor_lang::{prelude::AccountDeserialize as _, Space as _},
    mollusk_svm::{
        program::keyed_account_for_system_program,
        result::{Check, InstructionResult},
        Mollusk,
    },
    prism_protocol::{
        self,
        sdk::{
            address_finders::find_campaign_address,
            instruction_builders::build_initialize_campaign_ix,
        },
        state::CampaignV0,
        ID as PRISM_PROGRAM_ID,
    },
    solana_sdk::{
        account::Account as SolanaAccount,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program::ID as SYSTEM_PROGRAM_ID,
    },
};

#[test]
fn test_initialize_campaign_success() {
    // 1. Setup Mollusk and Program
    let mollusk = Mollusk::new(&PRISM_PROGRAM_ID, "prism_protocol");

    // 2. Test Data and Accounts
    let admin_keypair = Keypair::new();
    let admin_address = admin_keypair.pubkey();

    let test_fingerprint: [u8; 32] = [1; 32];
    let mint = Pubkey::new_unique();

    let (campaign_address, campaign_bump) = find_campaign_address(
        &admin_address, //
        &test_fingerprint,
    );

    // 3. Build the Instruction using your SDK
    let (ix, _, _) = build_initialize_campaign_ix(
        admin_address, //
        campaign_address,
        test_fingerprint,
        mint,
    )
    .expect("Failed to build initialize_campaign instruction");

    let keyed_account_for_admin = (
        admin_address,
        SolanaAccount::new(1_000_000_000, 0, &SYSTEM_PROGRAM_ID),
    );

    let keyed_account_for_campaign = (
        campaign_address,
        SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
    );

    // 5. Process and Validate Instruction
    println!(
        "Attempting to initialize campaign: {} (bump: {}, size: {}, admin: {})",
        campaign_address,
        campaign_bump,
        CampaignV0::INIT_SPACE,
        admin_address,
    );

    let result: InstructionResult = mollusk.process_and_validate_instruction(
        &ix,
        &[
            keyed_account_for_system_program(),
            keyed_account_for_admin,
            keyed_account_for_campaign,
        ],
        &[
            Check::success(),
            Check::account(&campaign_address)
                .executable(false)
                .lamports(1628640)
                .owner(&PRISM_PROGRAM_ID)
                .space(CampaignV0::INIT_SPACE + 8)
                .build(),
        ],
    );

    // 6. Further Validation of Account Data and CU (Benchmarking Start)
    println!(
        "Initialize Campaign CU consumed: {}",
        result.compute_units_consumed
    );

    let campaign_account = result
        .get_account(&campaign_address)
        .expect("Campaign PDA was not created");

    assert_eq!(
        // duplicates check above (on purpose)
        campaign_account.owner,
        PRISM_PROGRAM_ID,
        "owner mismatch: expected: {:?}, actual: {:?}",
        PRISM_PROGRAM_ID,
        campaign_account.owner
    );

    let campaign_state = CampaignV0::try_deserialize(&mut campaign_account.data.as_slice())
        .expect("Failed to deserialize Campaign state");

    assert_eq!(
        campaign_state.admin, admin_address,
        "admin mismatch: expected: {}, actual: {}",
        admin_address, campaign_state.admin
    );

    assert_eq!(
        campaign_state.fingerprint, test_fingerprint,
        "fingerprint mismatch: expected: {:?}, actual: {:?}",
        test_fingerprint, campaign_state.fingerprint
    );

    assert_eq!(
        campaign_state.mint, mint,
        "mint mismatch: expected: {:?}, actual: {:?}",
        mint, campaign_state.mint
    );

    assert_eq!(
        campaign_state.is_active, true,
        "is_active mismatch: expected: {}, actual: {}",
        true, campaign_state.is_active
    );

    assert_eq!(
        campaign_state.bump, campaign_bump,
        "bump mismatch: expected: {}, actual: {}",
        campaign_bump, campaign_state.bump
    );
}

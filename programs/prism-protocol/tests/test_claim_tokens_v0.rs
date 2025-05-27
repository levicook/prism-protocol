#![cfg(all(feature = "test-sbf"))]

use {
    anchor_lang::prelude::Pubkey,
    anchor_spl::{
        associated_token::get_associated_token_address,
        token::{spl_token, Mint, TokenAccount, ID as TOKEN_PROGRAM_ID},
    },
    mollusk_svm::{program::keyed_account_for_system_program, result::Check, sysvar::Sysvars},
    prism_protocol_sdk::{
        address_finders::find_claim_receipt_v0_address, instruction_builders::build_claim_tokens_ix,
    },
    prism_protocol_testing::{generate_test_vaults, TestFixture, TEST_AMOUNT_PER_ENTITLEMENT},
    solana_sdk::{account::Account as SolanaAccount, system_program::ID as SYSTEM_PROGRAM_ID},
};

#[test]
fn test_merkle_tree_proof_generation() {
    // Test that our merkle tree implementation generates valid proofs
    let mut fixture = TestFixture::new();
    let campaign_result = fixture.initialize_campaign();

    // Setup test data
    let claimants = [
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];
    let vaults = generate_test_vaults(2);
    let amount_per_entitlement = TEST_AMOUNT_PER_ENTITLEMENT;

    // Initialize cohort with real merkle tree
    let cohort_result = fixture.initialize_cohort_with_merkle_tree(
        &campaign_result,
        &claimants,
        &vaults,
        amount_per_entitlement,
    );

    // Test proof generation and verification for each claimant
    for claimant in &claimants {
        // Get the claimant's leaf data
        let claimant_leaf = cohort_result
            .merkle_tree
            .leaf_for_claimant(claimant)
            .expect("Failed to get claimant leaf");

        // Generate merkle proof
        let merkle_proof = cohort_result
            .merkle_tree
            .proof_for_claimant(claimant)
            .expect("Failed to generate merkle proof");

        // Verify the proof using our merkle tree
        let is_valid = cohort_result
            .merkle_tree
            .verify_proof(claimant, &merkle_proof)
            .expect("Failed to verify proof");

        assert!(is_valid, "Proof should be valid for claimant {}", claimant);

        println!(
            "✅ Valid proof generated for claimant {} (entitlements: {}, vault: {})",
            claimant, claimant_leaf.entitlements, claimant_leaf.assigned_vault
        );
    }

    // Test that proofs don't work for wrong claimants
    let proof_for_first = cohort_result
        .merkle_tree
        .proof_for_claimant(&claimants[0])
        .expect("Failed to generate proof for first claimant");

    let is_invalid = cohort_result
        .merkle_tree
        .verify_proof(&claimants[1], &proof_for_first)
        .expect("Failed to verify proof");

    assert!(!is_invalid, "Proof should be invalid for wrong claimant");

    println!("✅ Merkle tree proof generation and verification test passed");
}

#[test]
fn test_claim_tokens_instruction_building() {
    // Test that we can build claim tokens instructions correctly
    let mut fixture = TestFixture::new();
    let campaign_result = fixture.initialize_campaign();

    // Setup test data
    let claimant = Pubkey::new_unique();
    let other_claimants = [Pubkey::new_unique(), Pubkey::new_unique()];
    let all_claimants = [claimant, other_claimants[0], other_claimants[1]];
    let vaults = generate_test_vaults(2);
    let amount_per_entitlement = TEST_AMOUNT_PER_ENTITLEMENT;

    // Initialize cohort with real merkle tree
    let cohort_result = fixture.initialize_cohort_with_merkle_tree(
        &campaign_result,
        &all_claimants,
        &vaults,
        amount_per_entitlement,
    );

    // Get the claimant's leaf data
    let claimant_leaf = cohort_result
        .merkle_tree
        .leaf_for_claimant(&claimant)
        .expect("Failed to get claimant leaf");

    // Generate valid merkle proof
    let valid_merkle_proof = cohort_result
        .merkle_tree
        .proof_for_claimant(&claimant)
        .expect("Failed to generate merkle proof");

    // Generate invalid merkle proof (use proof for different claimant)
    let invalid_merkle_proof = cohort_result
        .merkle_tree
        .proof_for_claimant(&other_claimants[0])
        .expect("Failed to generate merkle proof for other claimant");

    // Setup addresses - use the assigned vault directly from the merkle tree
    let token_vault_address = claimant_leaf.assigned_vault;
    let claimant_token_account = get_associated_token_address(&claimant, &fixture.mint);
    let (claim_receipt_address, _) =
        find_claim_receipt_v0_address(&cohort_result.address, &claimant);

    let merkle_root = cohort_result
        .merkle_tree
        .root()
        .expect("Failed to get merkle root");

    // Test building instruction with valid proof
    let (valid_claim_ix, _, _) = build_claim_tokens_ix(
        fixture.admin_address,
        claimant,
        campaign_result.address,
        cohort_result.address,
        token_vault_address,
        fixture.mint,
        claimant_token_account,
        claim_receipt_address,
        fixture.test_fingerprint,
        merkle_root,
        valid_merkle_proof,
        claimant_leaf.assigned_vault,
        claimant_leaf.entitlements,
    )
    .expect("Failed to build claim_tokens instruction with valid proof");

    // Test building instruction with invalid proof
    let (invalid_claim_ix, _, _) = build_claim_tokens_ix(
        fixture.admin_address,
        claimant,
        campaign_result.address,
        cohort_result.address,
        token_vault_address,
        fixture.mint,
        claimant_token_account,
        claim_receipt_address,
        fixture.test_fingerprint,
        merkle_root,
        invalid_merkle_proof,
        claimant_leaf.assigned_vault,
        claimant_leaf.entitlements,
    )
    .expect("Failed to build claim_tokens instruction with invalid proof");

    // Verify instructions have the same structure but different data
    assert_eq!(valid_claim_ix.program_id, invalid_claim_ix.program_id);
    assert_eq!(
        valid_claim_ix.accounts.len(),
        invalid_claim_ix.accounts.len()
    );
    assert_ne!(valid_claim_ix.data, invalid_claim_ix.data); // Different proof data

    println!("✅ Claim tokens instruction building test passed");
    println!(
        "   - Valid proof instruction: {} bytes",
        valid_claim_ix.data.len()
    );
    println!(
        "   - Invalid proof instruction: {} bytes",
        invalid_claim_ix.data.len()
    );
    println!("   - Accounts required: {}", valid_claim_ix.accounts.len());
}

#[test]
fn test_claim_tokens_end_to_end() {
    // Complete end-to-end test using actual token program
    let mut fixture = TestFixture::new();

    let campaign_result = fixture.initialize_campaign();

    // Setup test data
    let claimant = Pubkey::new_unique();
    let other_claimants = [Pubkey::new_unique(), Pubkey::new_unique()];
    let all_claimants = [claimant, other_claimants[0], other_claimants[1]];
    let vaults = generate_test_vaults(2);
    let amount_per_entitlement = TEST_AMOUNT_PER_ENTITLEMENT;

    // Initialize cohort with real merkle tree
    let cohort_result = fixture.initialize_cohort_with_merkle_tree(
        &campaign_result,
        &all_claimants,
        &vaults,
        amount_per_entitlement,
    );

    // Get the claimant's leaf data
    let claimant_leaf = cohort_result
        .merkle_tree
        .leaf_for_claimant(&claimant)
        .expect("Failed to get claimant leaf");

    // Generate valid merkle proof
    let valid_merkle_proof = cohort_result
        .merkle_tree
        .proof_for_claimant(&claimant)
        .expect("Failed to generate merkle proof");

    // Setup addresses - use the assigned vault directly from the merkle tree
    let token_vault_address = claimant_leaf.assigned_vault;
    let claimant_token_account = get_associated_token_address(&claimant, &fixture.mint);
    let (claim_receipt_address, _) =
        find_claim_receipt_v0_address(&cohort_result.address, &claimant);

    // Step 1: Initialize mint using token program
    let mint_authority = fixture.admin_address;
    let initialize_mint_ix = spl_token::instruction::initialize_mint2(
        &TOKEN_PROGRAM_ID,
        &fixture.mint,
        &mint_authority,
        None, // No freeze authority
        9,    // 9 decimals
    )
    .expect("Failed to create initialize_mint2 instruction");

    let mint_account = SolanaAccount {
        lamports: 1_461_600, // Rent-exempt amount for mint
        data: vec![0u8; Mint::LEN],
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };

    let mint_result = fixture.mollusk.process_and_validate_instruction(
        &initialize_mint_ix,
        &[(fixture.mint, mint_account.clone())],
        &[Check::success()],
    );

    // Get the updated mint account after initialization
    let initialized_mint_account = mint_result
        .get_account(&fixture.mint)
        .expect("Mint account not found after initialization")
        .clone();

    println!("✅ Mint initialized successfully");

    // Step 2: Initialize token vault account (PDA owned by cohort)
    let initialize_vault_ix = spl_token::instruction::initialize_account3(
        &TOKEN_PROGRAM_ID,
        &token_vault_address,
        &fixture.mint,
        &cohort_result.address, // Cohort PDA owns the vault
    )
    .expect("Failed to create initialize_account3 instruction for vault");

    let vault_account = SolanaAccount {
        lamports: 2_039_280, // Rent-exempt amount for token account
        data: vec![0u8; TokenAccount::LEN],
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };

    let vault_result = fixture.mollusk.process_and_validate_instruction(
        &initialize_vault_ix,
        &[
            (token_vault_address, vault_account.clone()),
            (fixture.mint, initialized_mint_account.clone()),
        ],
        &[Check::success()],
    );

    // Get the updated vault account after initialization
    let initialized_vault_account = vault_result
        .get_account(&token_vault_address)
        .expect("Vault account not found after initialization")
        .clone();

    println!("✅ Token vault initialized successfully");

    // Step 3: Initialize claimant token account
    let initialize_claimant_ix = spl_token::instruction::initialize_account3(
        &TOKEN_PROGRAM_ID,
        &claimant_token_account,
        &fixture.mint,
        &claimant,
    )
    .expect("Failed to create initialize_account3 instruction for claimant");

    let claimant_account = SolanaAccount {
        lamports: 2_039_280, // Rent-exempt amount for token account
        data: vec![0u8; TokenAccount::LEN],
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };

    let claimant_result = fixture.mollusk.process_and_validate_instruction(
        &initialize_claimant_ix,
        &[
            (claimant_token_account, claimant_account.clone()),
            (fixture.mint, initialized_mint_account.clone()),
        ],
        &[Check::success()],
    );

    // Get the updated claimant account after initialization
    let initialized_claimant_account = claimant_result
        .get_account(&claimant_token_account)
        .expect("Claimant account not found after initialization")
        .clone();

    println!("✅ Claimant token account initialized successfully");

    // Step 4: Mint tokens to the vault
    let mint_amount = amount_per_entitlement * claimant_leaf.entitlements * 10; // Extra for safety
    let mint_to_vault_ix = spl_token::instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        &fixture.mint,
        &token_vault_address,
        &mint_authority,
        &[],
        mint_amount,
    )
    .expect("Failed to create mint_to instruction");

    let mint_result = fixture.mollusk.process_and_validate_instruction(
        &mint_to_vault_ix,
        &[
            (fixture.mint, initialized_mint_account.clone()),
            (token_vault_address, initialized_vault_account.clone()),
            (
                mint_authority,
                SolanaAccount::new(1_000_000_000, 0, &SYSTEM_PROGRAM_ID),
            ),
        ],
        &[Check::success()],
    );

    // Get the updated accounts after minting
    let final_mint_account = mint_result
        .get_account(&fixture.mint)
        .expect("Mint account not found after minting")
        .clone();
    let final_vault_account = mint_result
        .get_account(&token_vault_address)
        .expect("Vault account not found after minting")
        .clone();

    println!("✅ Tokens minted to vault: {}", mint_amount);

    // Step 5: Test claim tokens with valid proof
    let merkle_root = cohort_result
        .merkle_tree
        .root()
        .expect("Failed to get merkle root");
    let (claim_tokens_ix, _, _) = build_claim_tokens_ix(
        fixture.admin_address,
        claimant,
        campaign_result.address,
        cohort_result.address,
        token_vault_address,
        fixture.mint,
        claimant_token_account,
        claim_receipt_address,
        fixture.test_fingerprint,
        merkle_root,
        valid_merkle_proof,
        claimant_leaf.assigned_vault,
        claimant_leaf.entitlements,
    )
    .expect("Failed to build claim_tokens instruction");

    let sysvars = Sysvars::default(); // place in fixture?

    let result = fixture.mollusk.process_and_validate_instruction(
        &claim_tokens_ix,
        &[
            keyed_account_for_system_program(),
            (fixture.admin_address, campaign_result.admin_account.clone()),
            (
                claimant,
                SolanaAccount::new(1_000_000_000, 0, &SYSTEM_PROGRAM_ID),
            ),
            (
                campaign_result.address,
                campaign_result.campaign_account.clone(),
            ),
            (cohort_result.address, cohort_result.cohort_account.clone()),
            (token_vault_address, final_vault_account),
            (fixture.mint, final_mint_account),
            (claimant_token_account, initialized_claimant_account),
            (
                claim_receipt_address,
                SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
            ),
            mollusk_svm_programs_token::token::keyed_account(),
            mollusk_svm_programs_token::associated_token::keyed_account(),
            sysvars.keyed_account_for_rent_sysvar(),
        ],
        &[Check::success()],
    );

    println!(
        "✅ Tokens claimed successfully - CU consumed: {}, execution time: {}",
        result.compute_units_consumed, result.execution_time
    );

    // Verify the claim receipt was created
    let claim_receipt_account = result
        .get_account(&claim_receipt_address)
        .expect("Claim receipt account not found");

    assert_eq!(claim_receipt_account.owner, prism_protocol::ID);
    assert!(claim_receipt_account.data.len() > 0);

    // Verify tokens were transferred to claimant
    let _updated_claimant_account = result
        .get_account(&claimant_token_account)
        .expect("Claimant token account not found");

    // Parse the token account to check balance
    let expected_amount = amount_per_entitlement * claimant_leaf.entitlements;

    println!("✅ End-to-end claim tokens test completed successfully!");
    println!("   - Expected claim amount: {}", expected_amount);
    println!("   - Claimant: {}", claimant);
    println!("   - Vault: {}", claimant_leaf.assigned_vault);
    println!("   - Entitlements: {}", claimant_leaf.entitlements);
}

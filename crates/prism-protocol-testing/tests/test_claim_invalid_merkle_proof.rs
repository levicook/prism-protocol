use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::build_claim_tokens_v0_ix;
use prism_protocol_testing::{
    demand_prism_error, deterministic_keypair, FixtureStage, TestFixture,
};
use solana_message::Message;
use solana_signer::Signer as _;
use solana_transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;

/// Test claim with invalid merkle proof → InvalidMerkleProof
///
/// Should test:
/// - Set up active campaign with funded vault
/// - Create claimant with valid entitlements  
/// - Generate INVALID merkle proof (wrong proof, wrong leaf, etc.)
/// - Attempt claim_tokens_v0
/// - Verify fails with InvalidMerkleProof error
/// - Verify no tokens transferred
/// - Verify no ClaimReceipt created
#[ignore]
#[test]
fn test_claim_invalid_merkle_proof() {
    let mut test = TestFixture::default();

    // 1. Set up active campaign
    test.jump_to(FixtureStage::CampaignActivated);

    // 2. Advance past go-live slot to ensure we can reach merkle proof validation
    test.advance_slot_by(20); // Ensure we're past go-live slot

    // 3. Get valid claimant from compiled campaign
    let claimant_keypair = deterministic_keypair("early_adopter_1");
    let claimant_pubkey = claimant_keypair.pubkey();

    // 4. Extract claim data
    let mint = test.state.compiled_campaign.mint;

    let (cohort, leaf) = test
        .state
        .compiled_campaign
        .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
        .expect("early_adopter_1 should be in EarlyAdopters cohort");

    let vault = cohort
        .find_claimant_vault(&claimant_pubkey)
        .expect("Claimant should have assigned vault");

    let claimant_token_account = get_associated_token_address(&claimant_pubkey, &mint);

    test.airdrop(&claimant_pubkey, 1_000_000_000);

    // 5. Generate VALID proof first (we'll tamper with it)
    let valid_proof = {
        let (cohort, _) = test
            .state
            .compiled_campaign
            .find_claimant_in_cohort(&claimant_pubkey, "EarlyAdopters")
            .expect("early_adopter_1 should be in EarlyAdopters cohort");
        cohort
            .proof_for_claimant(&claimant_pubkey)
            .expect("Should be able to generate proof")
    };

    // Record balances before attempted claim
    let vault_balance_before = test
        .get_token_account_balance(&vault.address)
        .expect("Should be able to read vault balance");

    let claimant_balance_before = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0);

    println!("📊 Balances before invalid claim attempt:");
    println!(
        "  Vault: {}, Claimant: {}",
        vault_balance_before, claimant_balance_before
    );

    // 6. Generate INVALID proof (flip bits, use wrong leaf, truncate, etc.)
    let mut invalid_proof = valid_proof.clone();
    invalid_proof[0][0] = !invalid_proof[0][0];

    // 7. build_claim_tokens_v0_ix with invalid proof
    let (ix, _, _) = build_claim_tokens_v0_ix(
        &test.state.address_finder,
        test.state.compiled_campaign.admin,
        claimant_pubkey,
        mint,
        claimant_token_account,
        test.state.compiled_campaign.fingerprint,
        cohort.merkle_root,
        invalid_proof,
        leaf.assigned_vault_index,
        leaf.entitlements,
    )
    .expect("Failed to build claim tokens v0 ix");

    // 8. Expect transaction failure with InvalidMerkleProof error code
    let tx = Transaction::new(
        &[&claimant_keypair],
        Message::new(&[ix], Some(&claimant_pubkey)),
        test.latest_blockhash(),
    );

    let result = test.send_transaction(tx);

    demand_prism_error(
        result,
        PrismError::InvalidMerkleProof as u32,
        "InvalidMerkleProof",
    );

    // 9. Verify vault balance unchanged
    let vault_balance_after = test
        .get_token_account_balance(&vault.address)
        .expect("Should be able to read vault balance");
    assert_eq!(vault_balance_after, vault_balance_before);

    // 10. Verify claimant balance unchanged
    let claimant_balance_after = test
        .get_token_account_balance(&claimant_token_account)
        .unwrap_or(0);
    assert_eq!(claimant_balance_after, claimant_balance_before);

    // 11. Verify no ClaimReceipt PDA created
    let (claim_receipt_address, _) = test
        .state
        .address_finder
        .find_claim_receipt_v0_address(&cohort.address, &claimant_pubkey);

    assert!(!test.account_exists(&claim_receipt_address));
}

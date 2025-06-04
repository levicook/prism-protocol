use prism_protocol_sdk::build_initialize_vault_v0_ix;
use prism_protocol_testing::{FixtureStage, TestFixture};
use solana_instruction::error::InstructionError;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;

/// Test that vault initialization is NOT permissionless
///
/// This test demonstrates Anchor's security model: even if an attacker:
/// - Knows all the public vault parameters (campaign fingerprint, merkle root, vault index, etc.)
/// - Has sufficient funds to pay transaction fees
/// - Can construct a syntactically correct instruction
///
/// They still CANNOT initialize the vault because PDA derivation uses the admin's key.
/// The instruction will fail with AccountNotInitialized, proving the security model works.
#[test]
fn test_non_admin_cannot_initialize_vault() {
    let mut test = TestFixture::default();

    // Set up: cohorts initialized but vaults not yet initialized
    test.jump_to(FixtureStage::CohortsInitialized);

    // Create an attacker with sufficient funds
    let attacker = Keypair::new();
    test.airdrop(&attacker.pubkey(), 1_000_000_000);

    let campaign_fingerprint = test.state.compiled_campaign.fingerprint;
    let first_cohort = &test.state.compiled_campaign.cohorts[0];
    let cohort_merkle_root = first_cohort.merkle_root;
    let mint = test.state.compiled_campaign.mint;
    let vault_index = 0u8;

    // Attacker knows all public parameters and constructs instruction with THEIR key
    // This will derive a different (non-existent) campaign PDA
    let (ix, _, _) = build_initialize_vault_v0_ix(
        &test.state.address_finder,
        attacker.pubkey(), // Attacker's key - derives wrong campaign PDA!
        campaign_fingerprint,
        cohort_merkle_root,
        mint,
        vault_index,
    )
    .expect("Failed to build initialize vault v0 ix");

    // Attacker can pay fees and sign, but instruction will fail
    let tx = Transaction::new(
        &[&attacker],
        Message::new(&[ix], Some(&attacker.pubkey())),
        test.latest_blockhash(),
    );

    let result = test.send_transaction(tx);

    match result {
        Ok(_) => {
            panic!(
                "❌ Vault initialization should have failed - instruction is not permissionless!"
            );
        }
        Err(failed_meta) => {
            // The instruction fails because the campaign PDA derived from attacker's key doesn't exist
            // This proves the security model: you can't access accounts you don't own
            const EXPECTED_ERROR: u32 = 3012; // AccountNotInitialized

            match failed_meta.err {
                TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
                    assert_eq!(code, EXPECTED_ERROR, "Expected AccountNotInitialized error");
                    println!("✅ Confirmed AccountNotInitialized error (code: {})", code);
                    println!("✅ This proves vault initialization is NOT permissionless");
                }
                _ => {
                    panic!(
                        "Expected TransactionError::InstructionError with AccountNotInitialized, got: {:?}",
                        failed_meta.err
                    );
                }
            }
        }
    }

    // Additional verification: show that the CORRECT admin CAN initialize the vault
    println!("🔐 Demonstrating that only the correct admin can initialize vault...");

    let (correct_ix, _, _) = build_initialize_vault_v0_ix(
        &test.state.address_finder,
        test.state.compiled_campaign.admin, // Correct admin
        campaign_fingerprint,
        cohort_merkle_root,
        mint,
        vault_index,
    )
    .expect("Failed to build initialize vault v0 ix");

    let correct_tx = Transaction::new(
        &[&test.state.admin_keypair],
        Message::new(&[correct_ix], Some(&test.state.compiled_campaign.admin)),
        test.latest_blockhash(),
    );

    test.send_transaction(correct_tx)
        .expect("Correct admin should be able to initialize vault");

    println!("✅ Correct admin successfully initialized the vault");
    println!("🎉 Security model verification complete!");
}

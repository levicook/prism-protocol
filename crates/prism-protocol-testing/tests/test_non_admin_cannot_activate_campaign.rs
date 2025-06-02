use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::build_activate_campaign_v0_ix;
use prism_protocol_testing::{FixtureStage, TestFixture};
use solana_instruction::error::InstructionError;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;

/// Test campaign activation with wrong admin → CampaignAdminMismatch error
///
/// Should test:
/// - Set up full campaign flow (all components initialized and activated)
/// - Create a different admin keypair (wrong admin)
/// - Attempt activate_campaign_v0 with wrong admin signature
/// - Verify fails with CampaignAdminMismatch error code
/// - Ensure admin authorization is properly enforced
#[test]
#[ignore]
fn test_non_admin_cannot_activate_campaign() {
    let mut test = TestFixture::default();

    test.jump_to(FixtureStage::CampaignInitialized)
        .expect("campaign initialization failed");

    test.jump_to(FixtureStage::CohortInitialized)
        .expect("cohort initialization failed");

    test.jump_to(FixtureStage::VaultInitialized)
        .expect("vault initialization failed");

    test.jump_to(FixtureStage::VaultActivated)
        .expect("vault activation failed");

    let state = test
        .jump_to(FixtureStage::CohortActivated)
        .expect("cohort activation failed");

    // Now try to activate campaign with wrong admin
    let wrong_admin = Keypair::new();
    test.airdrop(&wrong_admin.pubkey(), 1_000_000_000)
        .expect("airdrop failed");

    let campaign_fingerprint = state
        .campaign_fingerprint
        .expect("campaign fingerprint not initialized");

    let (ix, _, _) = build_activate_campaign_v0_ix(
        &test.address_finder,
        wrong_admin.pubkey(), // Wrong admin!
        campaign_fingerprint,
        [1u8; 32],              // final_db_ipfs_hash (non-zero)
        test.latest_slot() + 1, // go_live_slot
    )
    .expect("Failed to build activate campaign v0 ix");

    let tx = Transaction::new(
        &[&wrong_admin],
        Message::new(&[ix], Some(&wrong_admin.pubkey())),
        test.latest_blockhash(),
    );

    let result = test.send_transaction(tx);

    match result {
        Ok(_) => {
            panic!("❌ Campaign activation should have failed with wrong admin!");
        }
        Err(failed_meta) => {
            // Verify we got the expected CampaignAdminMismatch error
            const EXPECTED_ERROR: u32 = PrismError::CampaignAdminMismatch as u32 + 6000; // Anchor offset

            match failed_meta.err {
                TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
                    assert_eq!(code, EXPECTED_ERROR, "Expected CampaignAdminMismatch error");
                    println!("✅ Confirmed CampaignAdminMismatch error (code: {})", code);
                }
                _ => {
                    panic!(
                        "Expected TransactionError::InstructionError with CampaignAdminMismatch, got: {:?}",
                        failed_meta.err
                    );
                }
            }
        }
    }

    println!("🎉 Non-admin campaign activation test passed!");
}

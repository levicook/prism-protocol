use prism_protocol_sdk::build_activate_campaign_v0_ix;
use prism_protocol_testing::{demand_account_not_initialized_error, FixtureStage, TestFixture};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

/// Test that campaign activation is NOT permissionless
///
/// This test demonstrates Anchor's security model: even if an attacker:
/// - Knows all the public campaign parameters (fingerprint, etc.)
/// - Has sufficient funds to pay transaction fees
/// - Can construct a syntactically correct instruction
///
/// They still CANNOT activate the campaign because PDA derivation uses the admin's key.
/// The instruction will fail with AccountNotInitialized, proving the security model works.
#[ignore]
#[test]
fn test_non_admin_cannot_activate_campaign() {
    let mut test = TestFixture::default();

    test.jump_to(FixtureStage::CohortsActivated);

    // Create an attacker with sufficient funds
    let attacker = Keypair::new();

    test.airdrop(&attacker.pubkey(), 1_000_000_000);

    let campaign_fingerprint = test.state.compiled_campaign.fingerprint;

    // Attacker knows all public parameters and constructs instruction with THEIR key
    // This will derive a different (non-existent) campaign PDA
    let (ix, _, _) = build_activate_campaign_v0_ix(
        &test.state.address_finder,
        attacker.pubkey(), // Attacker's key - derives wrong PDA!
        campaign_fingerprint,
        [1u8; 32],               // final_db_ipfs_hash (non-zero)
        test.current_slot() + 1, // go_live_slot
    )
    .expect("Failed to build activate campaign v0 ix");

    // Attacker can pay fees and sign, but instruction will fail
    let tx = Transaction::new(
        &[&attacker],
        Message::new(&[ix], Some(&attacker.pubkey())),
        test.latest_blockhash(),
    );

    let result = test.send_transaction(tx);
    demand_account_not_initialized_error(result);

    // Additional verification: show that the CORRECT admin CAN activate
    println!("🔐 Demonstrating that only the correct admin can activate...");

    let (correct_ix, _, _) = build_activate_campaign_v0_ix(
        &test.state.address_finder,
        test.state.compiled_campaign.admin, // Correct admin
        campaign_fingerprint,
        [1u8; 32],               // final_db_ipfs_hash (non-zero)
        test.current_slot() + 1, // go_live_slot
    )
    .expect("Failed to build activate campaign v0 ix");

    let correct_tx = Transaction::new(
        &[&test.state.admin_keypair],
        Message::new(&[correct_ix], Some(&test.state.compiled_campaign.admin)),
        test.latest_blockhash(),
    );

    test.send_transaction(correct_tx)
        .expect("Correct admin should be able to activate campaign");

    println!("✅ Correct admin successfully activated the campaign");
    println!("🎉 Security model verification complete!");
}

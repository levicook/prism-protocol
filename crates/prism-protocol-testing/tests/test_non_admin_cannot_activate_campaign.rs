use prism_protocol_sdk::build_activate_campaign_v0_ix;
use prism_protocol_testing::{FixtureStage, TestFixture};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

#[test]
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
        [0; 32],                // final_db_ipfs_hash
        test.latest_slot() + 1, // go_live_slot
    )
    .expect("Failed to build activate campaign v0 ix");

    let tx = Transaction::new(
        &[&wrong_admin],
        Message::new(&[ix], Some(&wrong_admin.pubkey())),
        test.latest_blockhash(),
    );

    let result = test.send_transaction(tx);
    assert!(
        result.is_err(),
        "Expected campaign activation to fail with wrong admin"
    );

    println!("✅ Correctly prevented non-admin from activating campaign");
}

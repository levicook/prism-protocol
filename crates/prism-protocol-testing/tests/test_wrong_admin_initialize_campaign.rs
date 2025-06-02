use prism_protocol_sdk::{build_initialize_campaign_v0_ix, AddressFinder};
use prism_protocol_testing::TestFixture;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

#[test]
fn test_wrong_admin_initialize_campaign() {
    let mut test = TestFixture::default();

    // Create a mint
    let decimals = 9;
    let (mint, _) = test.create_mint(decimals).expect("mint creation failed");

    // Create a different admin (not the fixture's admin)
    let wrong_admin = Keypair::new();
    let campaign_fingerprint = [1u8; 32];

    // Try to initialize campaign with wrong admin
    let address_finder = AddressFinder::default();
    let (ix, _, _) = build_initialize_campaign_v0_ix(
        &address_finder,
        wrong_admin.pubkey(), // Wrong admin!
        campaign_fingerprint,
        mint,
        1,
    )
    .expect("Failed to build initialize campaign v0 ix");

    let tx = Transaction::new(
        &[&wrong_admin], // Signed by wrong admin
        Message::new(&[ix], Some(&wrong_admin.pubkey())),
        test.latest_blockhash(),
    );

    // This should fail - wrong admin can't create campaigns
    let result = test.send_transaction(tx);
    assert!(
        result.is_err(),
        "Expected campaign initialization to fail with wrong admin"
    );

    println!("✅ Correctly prevented wrong admin from initializing campaign");
} 
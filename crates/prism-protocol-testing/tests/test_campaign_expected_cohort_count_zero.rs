// use prism_protocol_testing::TestFixture;

use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::{build_initialize_campaign_v0_ix, AddressFinder};
use prism_protocol_testing::demand_prism_error;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

/// Test campaign initialization with expected_cohort_count = 0 → NoCohortsExpected
///
/// This test validates that campaigns cannot be created with zero expected cohorts:
/// - Verifies proper input validation at the campaign initialization level  
/// - Ensures the business rule "campaigns must expect at least one cohort" is enforced
/// - Tests the NoCohortsExpected error is correctly triggered and returned
///
/// **Background**: Campaign initialization is the first step in the deployment process.
/// The expected_cohort_count parameter tells the system how many cohorts will be
/// created for this campaign. Setting it to 0 would create an invalid campaign
/// with no distribution mechanism, which the protocol correctly prevents.
#[tokio::test]
async fn test_campaign_expected_cohort_count_zero() {
    // Create minimal test environment
    let mut svm = LiteSVM::new();

    // Load the prism protocol program
    prism_protocol_testing::load_prism_protocol(&mut svm, prism_protocol::ID);

    // Create test keypairs
    let admin_keypair = Keypair::new();
    let campaign_keypair = Keypair::new();
    let mint_keypair = Keypair::new();

    // Give admin some SOL for transaction fees
    svm.airdrop(&admin_keypair.pubkey(), 1_000_000_000)
        .expect("Failed to airdrop to admin");

    // Create address finder for instruction building
    let address_finder = AddressFinder::new(
        admin_keypair.pubkey(),
        campaign_keypair.pubkey(),
        mint_keypair.pubkey(),
    );

    // Build campaign initialization instruction with expected_cohort_count = 0
    let (bad_campaign_ix, _, _) = build_initialize_campaign_v0_ix(
        &address_finder,
        0, // ← This should trigger NoCohortsExpected error
    )
    .expect("Failed to build initialize campaign v0 ix");

    // Create and send transaction with the problematic instruction
    let tx = Transaction::new(
        &[&admin_keypair, &campaign_keypair],
        Message::new(&[bad_campaign_ix], Some(&admin_keypair.pubkey())),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    // Verify fails with NoCohortsExpected error (code 6012)
    demand_prism_error(
        result,
        PrismError::NoCohortsExpected as u32,
        "NoCohortsExpected",
    );

    println!("✅ Campaign initialization correctly rejected zero expected_cohort_count");
}

use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_sdk::{build_initialize_cohort_v0_ix, AddressFinder};
use prism_protocol_testing::demand_prism_error;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

/// Test cohort initialization with expected_vault_count = 0 → NoVaultsExpected
///
/// This test validates that cohorts cannot be created with zero expected vaults:
/// - Verifies proper input validation at the cohort initialization level
/// - Ensures the business rule "cohorts must expect at least one vault" is enforced
/// - Tests the NoVaultsExpected error is correctly triggered and returned
/// - Demonstrates validation occurs even when campaign setup is valid
///
/// **Background**: Cohort initialization is the second step in campaign deployment.
/// The expected_vault_count parameter tells the system how many vaults will be
/// created for token distribution within this cohort. Setting it to 0 would create
/// a cohort with no distribution mechanism, which the protocol correctly prevents.
///
/// **Business rule enforced**: `expected_vault_count > 0` during cohort initialization
#[tokio::test]
async fn test_cohort_expected_vault_count_zero() {
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
        mint_keypair.pubkey()
    );

    // First initialize a valid campaign (cohorts need a campaign to exist)
    let (campaign_ix, _, _) = prism_protocol_sdk::build_initialize_campaign_v0_ix(
        &address_finder,
        1, // Valid expected_cohort_count
    )
    .expect("Failed to build initialize campaign v0 ix");

    let campaign_tx = Transaction::new(
        &[&admin_keypair, &campaign_keypair],
        Message::new(&[campaign_ix], Some(&admin_keypair.pubkey())),
        svm.latest_blockhash(),
    );

    svm.send_transaction(campaign_tx)
        .expect("Campaign initialization should succeed");

    // Now attempt cohort initialization with expected_vault_count = 0
    let merkle_root = [1u8; 32]; // Dummy merkle root
    let amount_per_entitlement = 1000; // Valid amount

    let (bad_cohort_ix, _, _) = build_initialize_cohort_v0_ix(
        &address_finder,
        merkle_root,
        amount_per_entitlement,
        0, // ← This should trigger NoVaultsExpected error
    )
    .expect("Failed to build initialize cohort v0 ix");

    // Create and send transaction with the problematic instruction
    let tx = Transaction::new(
        &[&admin_keypair],
        Message::new(&[bad_cohort_ix], Some(&admin_keypair.pubkey())),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    // Verify fails with NoVaultsExpected error (code 6013)
    demand_prism_error(
        result,
        PrismError::NoVaultsExpected as u32,
        "NoVaultsExpected",
    );

    println!("✅ Cohort initialization correctly rejected zero expected_vault_count");
}

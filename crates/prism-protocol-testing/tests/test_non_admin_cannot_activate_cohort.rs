use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use prism_protocol::ErrorCode;
use prism_protocol_sdk::CompiledCohortExt;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

/// Test that cohort activation requires proper admin authority
///
/// This test demonstrates the security model: even if an attacker:
/// - Knows all public addresses (campaign, cohort, etc.)
/// - Has sufficient funds to pay transaction fees
/// - Can construct a syntactically correct instruction with the right accounts
///
/// They still CANNOT activate the cohort because:
/// - The instruction checks that the signer matches the campaign's admin
/// - Attacker cannot sign as the admin (doesn't have the private key)
/// - The instruction fails with CampaignAdminMismatch error
#[tokio::test]
async fn test_non_admin_cannot_activate_cohort() {
    let state = FixtureState::default_v1().await;
    let mut test = TestFixture::new(state, LiteSVM::new()).await.unwrap();

    // Set up: vaults activated but cohorts not yet activated
    test.jump_to(FixtureStage::VaultsActivated).await;

    // Create an attacker with sufficient funds
    let attacker = Keypair::new();
    test.airdrop(&attacker.pubkey(), 1_000_000_000);

    // Get the legitimate accounts - this is what the attacker can observe
    let cohorts = test.state.compiled_cohorts().await;
    let first_cohort = &cohorts[0];
    let cohort_merkle_root = first_cohort.merkle_root();

    // Get legitimate addresses from the fixture
    let legitimate_campaign = test.state.campaign_address();
    let legitimate_cohort = first_cohort.address();

    // Attacker constructs instruction with correct accounts but wrong signer
    let malicious_accounts = prism_protocol::accounts::ActivateCohortV0 {
        admin: attacker.pubkey(),      // Attacker signs (NOT the real admin!)
        campaign: legitimate_campaign, // Correct campaign
        cohort: legitimate_cohort,     // Correct cohort
    };

    let ix_data = prism_protocol::instruction::ActivateCohortV0 { cohort_merkle_root };

    // Create malicious instruction with correct accounts but wrong signer
    let malicious_ix = Instruction {
        program_id: test.state.prism_program_id(),
        accounts: malicious_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    // Attacker tries to execute with their signature (not the admin's)
    let tx = Transaction::new(
        &[&attacker], // Attacker signs instead of admin!
        Message::new(&[malicious_ix], Some(&attacker.pubkey())),
        test.latest_blockhash(),
    );

    let result = test.send_transaction(tx);

    // Should fail with CampaignAdminMismatch because attacker is not the admin
    demand_prism_error(
        result,
        ErrorCode::CampaignAdminMismatch as u32,
        "CampaignAdminMismatch",
    );

    println!("✅ Attacker cannot sign as admin to activate cohort");

    // Additional verification: show that the CORRECT admin CAN activate the cohort
    println!("🔐 Demonstrating that only the correct admin can activate cohort...");

    // Use TestFixture's helper method which handles the correct signing
    test.try_activate_cohorts()
        .await
        .expect("Correct admin should be able to activate cohort");

    println!("✅ Correct admin successfully activated the cohort");
    println!("🎉 Security model verification complete!");
}

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

/// Test that cohort initialization requires proper admin authority
///
/// This test demonstrates the security model: even if an attacker:
/// - Knows all public addresses (campaign, cohort, etc.)
/// - Has sufficient funds to pay transaction fees
/// - Can construct a syntactically correct instruction with the right accounts
///
/// They still CANNOT initialize the cohort because:
/// - The instruction checks that the signer matches the campaign's admin
/// - Attacker cannot sign as the admin (doesn't have the private key)
/// - The instruction fails with CampaignAdminMismatch error
#[tokio::test]
async fn test_non_admin_cannot_initialize_cohort() {
    let state = FixtureState::default_v1().await;
    let mut test = TestFixture::new(state, LiteSVM::new()).await.unwrap();

    // Set up: campaign initialized but cohorts not yet initialized
    test.jump_to(FixtureStage::CampaignInitialized).await;

    // Create an attacker with sufficient funds
    let attacker = Keypair::new();
    test.airdrop(&attacker.pubkey(), 1_000_000_000);

    // Get the legitimate accounts - this is what the attacker can observe
    let cohorts = test.state.compiled_cohorts().await;
    let first_cohort = &cohorts[0];
    let cohort_merkle_root = first_cohort.merkle_root();
    let amount_per_entitlement = first_cohort.amount_per_entitlement_token();
    let expected_vault_count = first_cohort.vault_count();

    // Get legitimate addresses from the fixture
    let legitimate_campaign = test.state.campaign_address();
    let (legitimate_cohort, _) = test
        .state
        .address_finder()
        .find_cohort_v0_address(&cohort_merkle_root);

    // Attacker constructs instruction with correct accounts but wrong signer
    let malicious_accounts = prism_protocol::accounts::InitializeCohortV0 {
        admin: attacker.pubkey(),      // Attacker signs (NOT the real admin!)
        campaign: legitimate_campaign, // Correct campaign
        cohort: legitimate_cohort,     // Correct cohort
        system_program: anchor_lang::system_program::ID,
    };

    let ix_data = prism_protocol::instruction::InitializeCohortV0 {
        merkle_root: cohort_merkle_root,
        amount_per_entitlement,
        expected_vault_count,
    };

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

    println!("✅ Attacker cannot sign as admin to initialize cohort");

    // Additional verification: show that the CORRECT admin CAN initialize the cohort
    println!("🔐 Demonstrating that only the correct admin can initialize cohort...");

    // Use TestFixture's helper method which handles the correct signing
    test.try_initialize_cohorts()
        .await
        .expect("Correct admin should be able to initialize cohort");

    println!("✅ Correct admin successfully initialized the cohort");
    println!("🎉 Security model verification complete!");
}

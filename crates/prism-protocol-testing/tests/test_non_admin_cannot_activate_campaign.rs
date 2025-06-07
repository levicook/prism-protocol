use litesvm::LiteSVM;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

/// Test that campaign activation requires proper admin authority
///
/// This test demonstrates the security model: even if an attacker:
/// - Knows all public addresses (campaign, etc.)  
/// - Has sufficient funds to pay transaction fees
/// - Can construct a syntactically correct instruction with the right accounts
///
/// They still CANNOT activate the campaign because:
/// - The instruction checks that the signer matches the campaign's admin
/// - Attacker cannot sign as the admin (doesn't have the private key)
/// - The instruction fails with CampaignAdminMismatch error
#[tokio::test]
async fn test_non_admin_cannot_activate_campaign() {
    let state = FixtureState::default_v1().await;
    let mut test = TestFixture::new(state, LiteSVM::new()).await.unwrap();

    test.jump_to(FixtureStage::CohortsActivated).await;

    // Create an attacker with sufficient funds
    let attacker = Keypair::new();

    test.airdrop(&attacker.pubkey(), 1_000_000_000);

    // Attacker constructs instruction with correct accounts but wrong signer
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_instruction::Instruction;

    // Get the legitimate campaign address that the attacker can observe
    let legitimate_campaign = test.state.campaign_address();

    // Attacker builds instruction with correct accounts but sets admin to themselves
    let malicious_accounts = prism_protocol::accounts::ActivateCampaignV0 {
        admin: attacker.pubkey(),      // Attacker signs (NOT the real admin!)
        campaign: legitimate_campaign, // Correct campaign address
    };

    let ix_data = prism_protocol::instruction::ActivateCampaignV0 {
        final_db_ipfs_hash: [1u8; 32],
        go_live_slot: test.current_slot() + 1,
    };

    // Create malicious instruction with correct accounts but wrong signer
    let ix = Instruction {
        program_id: test.state.prism_program_id(),
        accounts: malicious_accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    // Attacker can pay fees and sign, but instruction will fail
    let tx = Transaction::new(
        &[&attacker],
        Message::new(&[ix], Some(&attacker.pubkey())),
        test.latest_blockhash(),
    );

    let result = test.send_transaction(tx);

    // Should fail with CampaignAdminMismatch because attacker is not the admin
    use prism_protocol::ErrorCode;
    demand_prism_error(
        result,
        ErrorCode::CampaignAdminMismatch as u32,
        "CampaignAdminMismatch",
    );

    println!("✅ Attacker cannot sign as admin to activate campaign");

    // Additional verification: show that the CORRECT admin CAN activate
    println!("🔐 Demonstrating that only the correct admin can activate...");

    // Use the TestFixture's helper method which handles the correct signing
    test.try_activate_campaign_with_args(Some([1u8; 32]), Some(test.current_slot() + 1))
        .await
        .expect("Correct admin should be able to activate campaign");

    println!("✅ Correct admin successfully activated the campaign");
    println!("🎉 Security model verification complete!");
}

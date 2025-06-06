use anchor_lang::prelude::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use prism_protocol::{ErrorCode, VAULT_SEED_PREFIX};
use prism_protocol_sdk::CompiledCohortExt;
use prism_protocol_testing::{demand_prism_error, FixtureStage, FixtureState, TestFixture};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

/// Test that vault initialization requires proper admin authority
///
/// This test demonstrates the security model: even if an attacker:
/// - Knows all public addresses (campaign, cohort, vault, etc.)
/// - Has sufficient funds to pay transaction fees
/// - Can construct a syntactically correct instruction with the right accounts
///
/// They still CANNOT initialize the vault because:
/// - The instruction checks that the signer matches the campaign's admin
/// - Attacker cannot sign as the admin (doesn't have the private key)
/// - The instruction fails with admin mismatch error
#[tokio::test]
async fn test_non_admin_cannot_initialize_vault() {
    let state = FixtureState::rand().await;
    let mut test = TestFixture::new(state, LiteSVM::new())
        .await
        .expect("Failed to create test fixture");

    // Set up: cohorts initialized but vaults not yet initialized
    test.jump_to(FixtureStage::CohortsInitialized).await;

    // Create an attacker with sufficient funds
    let attacker = Keypair::new();
    test.airdrop(&attacker.pubkey(), 1_000_000_000);

    // Get the legitimate accounts - this is what the attacker can observe
    let cohorts = test.state.compiled_cohorts().await;
    let first_cohort = &cohorts[0];
    let cohort_merkle_root = first_cohort.merkle_root();
    let vault_index = 0u8;

    // Get legitimate addresses from the fixture
    let legitimate_campaign = test.state.address_finder().campaign;
    let legitimate_cohort = first_cohort.address();
    let legitimate_mint = test.state.address_finder().mint;

    // Derive the legitimate vault address
    let (legitimate_vault, _) = Pubkey::find_program_address(
        &[
            VAULT_SEED_PREFIX,
            legitimate_cohort.as_ref(),
            &vault_index.to_le_bytes(),
        ],
        &prism_protocol::ID,
    );

    // Attacker constructs instruction with correct accounts but wrong signer
    let malicious_accounts = prism_protocol::accounts::InitializeVaultV0 {
        admin: attacker.pubkey(),      // Attacker signs (NOT the real admin!)
        campaign: legitimate_campaign, // Correct campaign
        cohort: legitimate_cohort,     // Correct cohort
        mint: legitimate_mint,         // Correct mint
        vault: legitimate_vault,       // Correct vault
        token_program: spl_token::ID,
        system_program: anchor_lang::system_program::ID,
    };

    // Create instruction data
    let ix_data = prism_protocol::instruction::InitializeVaultV0 {
        cohort_merkle_root,
        vault_index,
    };

    // Create malicious instruction with correct accounts but wrong signer
    let malicious_ix = Instruction {
        program_id: prism_protocol::ID,
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

    // Use the demand_prism_error helper with the proper constant
    demand_prism_error(
        result,
        ErrorCode::CampaignAdminMismatch as u32,
        "CampaignAdminMismatch",
    );

    println!("✅ Attacker cannot sign as admin to initialize vaults");

    // Verification: our legitimate admin CAN initialize the vault
    println!("🔐 Demonstrating that the legitimate admin can initialize vault...");

    // Use our TestFixture's helper which builds the correct instruction
    test.try_initialize_vaults()
        .await
        .expect("Legitimate admin should be able to initialize vault");

    println!("✅ Legitimate admin successfully initialized the vault");
    println!("🎉 Security model verification complete!");
}

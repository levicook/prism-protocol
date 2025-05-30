use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_db::{CampaignDatabase, EligibilityInfo};
use prism_protocol_sdk::{build_claim_tokens_ix, AddressFinder};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use std::path::PathBuf;

#[derive(Debug)]
struct ClaimTransaction {
    eligibility: EligibilityInfo,
    claim_ix: Instruction,
    expected_tokens: u64,
}

pub fn execute(
    campaign_db_path: PathBuf,
    claimant_keypair_path: PathBuf,
    rpc_url: String,
    dry_run: bool,
) -> CliResult<()> {
    println!("🎯 Starting token claim process...");

    // Open database and RPC client
    let mut db = CampaignDatabase::open(&campaign_db_path)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to open database: {}", e)))?;
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Load claimant keypair
    println!("🔑 Loading claimant keypair...");
    let claimant_keypair = read_keypair_file(&claimant_keypair_path)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read keypair: {}", e)))?;
    let claimant_pubkey = claimant_keypair.pubkey();
    println!("✅ Claimant: {}", claimant_pubkey);

    // Get campaign info
    println!("📊 Reading campaign data...");
    let campaign_info = db
        .read_campaign_info()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read campaign: {}", e)))?;
    println!("✅ Campaign: {}", hex::encode(campaign_info.fingerprint));
    println!("   Mint: {}", campaign_info.mint);
    println!("   Admin: {}", campaign_info.admin);

    // Find eligible claims
    println!("🔍 Finding eligible claims...");
    let eligibility_info = db
        .read_claimant_eligibility(&claimant_pubkey)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read eligibility: {}", e)))?;

    // Filter out already claimed cohorts
    let pending_claims: Vec<&EligibilityInfo> = eligibility_info
        .iter()
        .filter(|info| !info.db_claimed)
        .collect();

    if pending_claims.is_empty() {
        println!(
            "❌ No pending claims found for claimant {}",
            claimant_pubkey
        );
        if !eligibility_info.is_empty() {
            println!("   (All eligible cohorts have already been claimed)");
        }
        return Ok(());
    }

    println!("✅ Found {} pending claim(s):", pending_claims.len());
    for (i, info) in pending_claims.iter().enumerate() {
        println!(
            "   {}. Cohort: {} - {} tokens ({} entitlements × {})",
            i + 1,
            info.cohort_name,
            info.total_tokens,
            info.entitlements,
            info.amount_per_entitlement
        );
    }

    // Get claimant's token account
    let claimant_token_account =
        get_associated_token_address(&claimant_pubkey, &campaign_info.mint);
    println!("💰 Claimant token account: {}", claimant_token_account);

    // Build claim transactions
    println!("🔨 Building claim transactions...");
    let claim_transactions = build_claim_transactions(
        &db,
        &rpc_client,
        &campaign_info,
        &claimant_pubkey,
        &claimant_token_account,
        &pending_claims,
    )?;

    // Process claims
    let mut successful_claims = 0;
    let mut total_tokens_claimed = 0u64;

    for (i, claim_tx) in claim_transactions.iter().enumerate() {
        println!(
            "\n📦 Processing claim {} of {}...",
            i + 1,
            claim_transactions.len()
        );
        println!("   Cohort: {}", claim_tx.eligibility.cohort_name);

        match execute_claim_transaction(&rpc_client, &claimant_keypair, claim_tx, dry_run) {
            Ok(signature) => {
                successful_claims += 1;
                total_tokens_claimed += claim_tx.expected_tokens;
                println!(
                    "✅ Successfully claimed {} tokens",
                    claim_tx.expected_tokens
                );

                if !dry_run {
                    // Update database to mark claim as processed
                    if let Err(e) = db.update_claim_status(
                        &claimant_pubkey,
                        &claim_tx.eligibility.cohort_name,
                        &signature,
                    ) {
                        println!("⚠️  Warning: Failed to update database: {}", e);
                    }
                }
            }
            Err(e) => {
                println!(
                    "❌ Failed to claim from cohort {}: {}",
                    claim_tx.eligibility.cohort_name, e
                );
                // Continue processing other claims
            }
        }
    }

    // Summary
    println!("\n🎉 Claim process completed!");
    println!(
        "   Successful claims: {}/{}",
        successful_claims,
        claim_transactions.len()
    );
    println!("   Total tokens claimed: {}", total_tokens_claimed);

    if dry_run {
        println!("   (This was a dry run - no transactions were submitted)");
    }

    if successful_claims < claim_transactions.len() {
        println!(
            "⚠️  Some claims failed. You can retry this command to attempt failed claims again."
        );
    }

    Ok(())
}

fn build_claim_transactions(
    db: &CampaignDatabase,
    rpc_client: &RpcClient,
    campaign_info: &prism_protocol_db::CampaignInfo,
    claimant: &Pubkey,
    claimant_token_account: &Pubkey,
    pending_claims: &[&EligibilityInfo],
) -> CliResult<Vec<ClaimTransaction>> {
    let address_finder = AddressFinder::default();
    let mut transactions = Vec::new();

    for eligibility in pending_claims {
        // Get merkle proof from database
        let proof_data = db
            .read_merkle_proof(claimant, &eligibility.cohort_name)
            .map_err(|e| CliError::InvalidConfig(format!("Failed to read merkle proof: {}", e)))?;

        // Calculate addresses
        let (campaign_address, _) = address_finder
            .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);

        let (cohort_address, _) = address_finder
            .find_cohort_v0_address(&campaign_address, &eligibility.cohort_merkle_root);

        let (claim_receipt_address, _) =
            address_finder.find_claim_receipt_v0_address(&cohort_address, claimant);

        // Check if already claimed on-chain (double-check against database)
        if rpc_client.get_account(&claim_receipt_address).is_ok() {
            println!(
                "⚠️  Cohort {} already claimed on-chain, skipping",
                eligibility.cohort_name
            );
            continue;
        }

        // Parse merkle proof from database format
        let merkle_proof: Vec<[u8; 32]> = proof_data
            .merkle_proof
            .iter()
            .map(|hex_str| {
                let bytes = hex::decode(hex_str.trim())
                    .map_err(|e| CliError::InvalidConfig(format!("Invalid proof hex: {}", e)))?;
                let hash: [u8; 32] = bytes.try_into().map_err(|_| {
                    CliError::InvalidConfig("Proof hash must be 32 bytes".to_string())
                })?;
                Ok(hash)
            })
            .collect::<CliResult<Vec<[u8; 32]>>>()?;

        // Get vault assignment from database (the correct, stored assignment)
        let (vault_index, assigned_vault) = db
            .read_claimant_vault_assignment(claimant, &eligibility.cohort_name)
            .map_err(|e| {
                CliError::InvalidConfig(format!("Failed to read vault assignment: {}", e))
            })?;

        // Build claim instruction
        let (claim_ix, _, _) = build_claim_tokens_ix(
            campaign_info.admin,
            *claimant,
            campaign_address,
            cohort_address,
            assigned_vault,
            campaign_info.mint,
            *claimant_token_account,
            claim_receipt_address,
            campaign_info.fingerprint,
            eligibility.cohort_merkle_root,
            merkle_proof,
            vault_index,
            eligibility.entitlements,
        )
        .map_err(|e| {
            CliError::InvalidConfig(format!("Failed to build claim instruction: {}", e))
        })?;

        transactions.push(ClaimTransaction {
            eligibility: (*eligibility).clone(),
            claim_ix,
            expected_tokens: eligibility.total_tokens,
        });
    }

    Ok(transactions)
}

fn execute_claim_transaction(
    rpc_client: &RpcClient,
    claimant_keypair: &Keypair,
    claim_tx: &ClaimTransaction,
    dry_run: bool,
) -> CliResult<String> {
    if dry_run {
        println!(
            "   Dry run: Would claim {} tokens",
            claim_tx.expected_tokens
        );
        return Ok(String::new());
    }

    // Build transaction using Message API for proper structure
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let message = Message::new(
        &[claim_tx.claim_ix.clone()],
        Some(&claimant_keypair.pubkey()),
    );
    let mut transaction = Transaction::new_unsigned(message);
    transaction.message.recent_blockhash = recent_blockhash;

    // Sign transaction
    transaction.sign(&[claimant_keypair], recent_blockhash);

    // Submit transaction with proper configuration
    let config = RpcSendTransactionConfig {
        skip_preflight: false,
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        encoding: None,
        max_retries: Some(5),
        min_context_slot: None,
    };

    let signature = rpc_client.send_and_confirm_transaction_with_spinner_and_config(
        &transaction,
        CommitmentConfig::confirmed(),
        config,
    )?;

    println!("   Transaction signature: {}", signature);
    Ok(signature.to_string())
}

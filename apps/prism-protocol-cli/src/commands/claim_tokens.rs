use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_sdk::{instruction_builders::build_claim_tokens_ix, AddressFinder};
use rusqlite::Connection;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
struct CampaignData {
    fingerprint: [u8; 32],
    mint: Pubkey,
    admin: Pubkey,
}

#[derive(Debug)]
struct ClaimData {
    cohort_name: String,
    cohort_merkle_root: [u8; 32],
    entitlements: u64,
    assigned_vault_index: u8,
    assigned_vault_pubkey: Pubkey,
    merkle_proof: Vec<[u8; 32]>,
    amount_per_entitlement: u64,
}

pub fn execute(
    campaign_db_in: PathBuf,
    claimant_keypair: PathBuf,
    rpc_url: String,
    dry_run: bool,
) -> CliResult<()> {
    println!("🎯 Starting token claim process...");

    // Load claimant keypair
    println!("🔑 Loading claimant keypair...");
    let claimant_keypair = read_keypair_file(&claimant_keypair)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read keypair: {}", e)))?;
    let claimant_pubkey = claimant_keypair.pubkey();
    println!("✅ Claimant: {}", claimant_pubkey);

    // Read campaign data from database
    println!("📊 Reading campaign data...");
    let campaign_data = read_campaign_data(&campaign_db_in)?;
    println!("✅ Campaign: {}", hex::encode(campaign_data.fingerprint));
    println!("   Mint: {}", campaign_data.mint);
    println!("   Admin: {}", campaign_data.admin);

    // Find all eligible claims for this claimant
    println!("🔍 Finding eligible claims...");
    let claims = find_claimant_claims(&campaign_db_in, &claimant_pubkey)?;

    if claims.is_empty() {
        println!(
            "❌ No eligible claims found for claimant {}",
            claimant_pubkey
        );
        return Ok(());
    }

    println!("✅ Found {} eligible claim(s):", claims.len());
    for (i, claim) in claims.iter().enumerate() {
        let total_tokens = claim.entitlements * claim.amount_per_entitlement;
        println!(
            "   {}. Cohort: {} - {} tokens ({} entitlements × {})",
            i + 1,
            claim.cohort_name,
            total_tokens,
            claim.entitlements,
            claim.amount_per_entitlement
        );
    }

    // Setup RPC client
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Get claimant's token account
    let claimant_token_account = get_associated_token_address(
        &claimant_pubkey, //
        &campaign_data.mint,
    );
    println!("💰 Claimant token account: {}", claimant_token_account);

    // Process each claim
    let mut successful_claims = 0;
    let mut total_tokens_claimed = 0u64;

    for (i, claim) in claims.iter().enumerate() {
        println!("\n📦 Processing claim {} of {}...", i + 1, claims.len());
        println!("   Cohort: {}", claim.cohort_name);

        match process_single_claim(
            &rpc_client,
            &claimant_keypair,
            &campaign_data,
            claim,
            &claimant_token_account,
            dry_run,
        ) {
            Ok((tokens_claimed, signature)) => {
                successful_claims += 1;
                total_tokens_claimed += tokens_claimed;
                println!("✅ Successfully claimed {} tokens", tokens_claimed);

                if !dry_run {
                    // Update database to mark claim as processed
                    if let Err(e) = mark_claim_processed(
                        &campaign_db_in,
                        &claimant_pubkey,
                        &claim.cohort_name,
                        &signature,
                    ) {
                        println!("⚠️  Warning: Failed to update database: {}", e);
                    }
                }
            }
            Err(e) => {
                println!(
                    "❌ Failed to claim from cohort {}: {}",
                    claim.cohort_name, e
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
        claims.len()
    );
    println!("   Total tokens claimed: {}", total_tokens_claimed);

    if dry_run {
        println!("   (This was a dry run - no transactions were submitted)");
    }

    if successful_claims < claims.len() {
        println!(
            "⚠️  Some claims failed. You can retry this command to attempt failed claims again."
        );
    }

    Ok(())
}

fn read_campaign_data(db_path: &PathBuf) -> CliResult<CampaignData> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare("SELECT fingerprint, mint, admin FROM campaign LIMIT 1")?;
    let mut rows = stmt.query_map([], |row| {
        let fingerprint_hex: String = row.get(0)?;
        let mint_str: String = row.get(1)?;
        let admin_str: String = row.get(2)?;
        Ok((fingerprint_hex, mint_str, admin_str))
    })?;

    if let Some(row) = rows.next() {
        let (fingerprint_hex, mint_str, admin_str) = row?;

        let fingerprint_bytes = hex::decode(fingerprint_hex)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid fingerprint hex: {}", e)))?;
        let fingerprint: [u8; 32] = fingerprint_bytes
            .try_into()
            .map_err(|_| CliError::InvalidConfig("Fingerprint must be 32 bytes".to_string()))?;

        let mint = Pubkey::from_str(&mint_str)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid mint pubkey: {}", e)))?;

        let admin = Pubkey::from_str(&admin_str)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid admin pubkey: {}", e)))?;

        Ok(CampaignData {
            fingerprint,
            mint,
            admin,
        })
    } else {
        Err(CliError::InvalidConfig(
            "No campaign data found in database".to_string(),
        ))
    }
}

fn find_claimant_claims(db_path: &PathBuf, claimant: &Pubkey) -> CliResult<Vec<ClaimData>> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT 
            c.cohort_name,
            c.entitlements,
            c.assigned_vault_index,
            c.assigned_vault_pubkey,
            c.merkle_proof,
            h.merkle_root,
            h.amount_per_entitlement
         FROM claimants c
         JOIN cohorts h ON c.cohort_name = h.cohort_name
         WHERE c.claimant = ? AND c.claimed_at IS NULL",
    )?;

    let rows = stmt.query_map([claimant.to_string()], |row| {
        let cohort_name: String = row.get(0)?;
        let entitlements: u64 = row.get(1)?;
        let assigned_vault_index: u8 = row.get(2)?;
        let assigned_vault_pubkey_str: String = row.get(3)?;
        let merkle_proof_hex: String = row.get(4)?;
        let merkle_root_hex: String = row.get(5)?;
        let amount_per_entitlement: u64 = row.get(6)?;

        Ok((
            cohort_name,
            entitlements,
            assigned_vault_index,
            assigned_vault_pubkey_str,
            merkle_proof_hex,
            merkle_root_hex,
            amount_per_entitlement,
        ))
    })?;

    let mut claims = Vec::new();
    for row in rows {
        let (
            cohort_name,
            entitlements,
            assigned_vault_index,
            assigned_vault_pubkey_str,
            merkle_proof_hex,
            merkle_root_hex,
            amount_per_entitlement,
        ) = row?;

        // Parse vault pubkey
        let assigned_vault_pubkey = Pubkey::from_str(&assigned_vault_pubkey_str)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid vault pubkey: {}", e)))?;

        // Parse merkle root
        let merkle_root_bytes = hex::decode(merkle_root_hex)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid merkle root hex: {}", e)))?;
        let cohort_merkle_root: [u8; 32] = merkle_root_bytes
            .try_into()
            .map_err(|_| CliError::InvalidConfig("Merkle root must be 32 bytes".to_string()))?;

        // Parse merkle proof
        let merkle_proof = if merkle_proof_hex.is_empty() {
            Vec::new()
        } else {
            merkle_proof_hex
                .split(',')
                .map(|hex_str| {
                    let bytes = hex::decode(hex_str.trim()).map_err(|e| {
                        CliError::InvalidConfig(format!("Invalid proof hex: {}", e))
                    })?;
                    let hash: [u8; 32] = bytes.try_into().map_err(|_| {
                        CliError::InvalidConfig("Proof hash must be 32 bytes".to_string())
                    })?;
                    Ok(hash)
                })
                .collect::<CliResult<Vec<[u8; 32]>>>()?
        };

        claims.push(ClaimData {
            cohort_name,
            cohort_merkle_root,
            entitlements,
            assigned_vault_index,
            assigned_vault_pubkey,
            merkle_proof,
            amount_per_entitlement,
        });
    }

    Ok(claims)
}

fn process_single_claim(
    rpc_client: &RpcClient,
    claimant_keypair: &Keypair,
    campaign_data: &CampaignData,
    claim: &ClaimData,
    claimant_token_account: &Pubkey,
    dry_run: bool,
) -> CliResult<(u64, String)> {
    let address_finder = AddressFinder::default();

    // Calculate addresses
    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

    let (cohort_address, _) =
        address_finder.find_cohort_v0_address(&campaign_address, &claim.cohort_merkle_root);

    let (claim_receipt_address, _) =
        address_finder.find_claim_receipt_v0_address(&cohort_address, &claimant_keypair.pubkey());

    // Check if already claimed
    if rpc_client.get_account(&claim_receipt_address).is_ok() {
        return Err(CliError::InvalidConfig(
            "Tokens already claimed from this cohort".to_string(),
        ));
    }

    // Build claim instruction
    let (claim_ix, _, _) = build_claim_tokens_ix(
        campaign_data.admin,
        claimant_keypair.pubkey(),
        campaign_address,
        cohort_address,
        claim.assigned_vault_pubkey,
        campaign_data.mint,
        *claimant_token_account,
        claim_receipt_address,
        campaign_data.fingerprint,
        claim.cohort_merkle_root,
        claim.merkle_proof.clone(),
        claim.assigned_vault_index,
        claim.entitlements,
    )
    .map_err(|e| CliError::InvalidConfig(format!("Failed to build claim instruction: {}", e)))?;

    let tokens_to_claim = claim.entitlements * claim.amount_per_entitlement;

    if dry_run {
        println!("   Dry run: Would claim {} tokens", tokens_to_claim);
        return Ok((tokens_to_claim, String::new()));
    }

    // Create transaction
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[claim_ix],
        Some(&claimant_keypair.pubkey()),
        &[claimant_keypair],
        recent_blockhash,
    );

    // Submit transaction
    let config = RpcSendTransactionConfig {
        skip_preflight: false,
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        encoding: None,
        max_retries: Some(5),
        min_context_slot: None,
    };

    let signature = rpc_client.send_and_confirm_transaction_with_spinner_and_config(
        &tx,
        CommitmentConfig::confirmed(),
        config,
    )?;

    println!("   Transaction signature: {}", signature);
    Ok((tokens_to_claim, signature.to_string()))
}

fn mark_claim_processed(
    db_path: &PathBuf,
    claimant: &Pubkey,
    cohort_name: &str,
    signature: &str,
) -> CliResult<()> {
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().timestamp();

    // Try to update with signature first (new schema)
    let result = conn.execute(
        "UPDATE claimants SET claimed_at = ?, claimed_signature = ? WHERE claimant = ? AND cohort_name = ?",
        (now, signature, claimant.to_string(), cohort_name),
    );

    // If that fails (old schema without claimed_signature column), fall back to just timestamp
    if result.is_err() {
        conn.execute(
            "UPDATE claimants SET claimed_at = ? WHERE claimant = ? AND cohort_name = ?",
            (now, claimant.to_string(), cohort_name),
        )?;
    } else {
        result?;
    }

    Ok(())
}

use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_sdk::{
    address_finders::{find_campaign_address, find_cohort_v0_address},
    instruction_builders::{build_initialize_campaign_ix, build_initialize_cohort_ix},
};
use rusqlite::Connection;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
struct CampaignData {
    fingerprint: [u8; 32],
    mint: Pubkey,
    admin: Pubkey,
}

#[derive(Debug)]
struct CohortData {
    name: String,
    merkle_root: [u8; 32],
    amount_per_entitlement: u64,
    vaults: Vec<Pubkey>,
}

pub fn execute(campaign_db_in: PathBuf, admin_keypair: PathBuf, rpc_url: String) -> CliResult<()> {
    println!("🚀 Deploying campaign on-chain...");
    println!("Database: {}", campaign_db_in.display());
    println!("Admin keypair: {}", admin_keypair.display());
    println!("RPC URL: {}", rpc_url);

    // Step 1: Read admin keypair
    println!("\n🔑 Reading admin keypair...");
    let admin_keypair = read_keypair_file(&admin_keypair)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read admin keypair: {}", e)))?;
    let admin_pubkey = admin_keypair.pubkey();
    println!("✅ Admin public key: {}", admin_pubkey);

    // Step 2: Connect to Solana RPC
    println!("\n🌐 Connecting to Solana RPC...");
    let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());

    // Test connection
    let _version = rpc_client
        .get_version()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to connect to RPC: {}", e)))?;
    println!("✅ Connected to Solana RPC: {}", rpc_url);

    // Step 3: Read campaign data from database
    println!("\n📋 Reading campaign data from database...");
    let campaign_data = read_campaign_data(&campaign_db_in)?;
    println!(
        "✅ Campaign fingerprint: {}",
        hex::encode(campaign_data.fingerprint)
    );
    println!("✅ Mint: {}", campaign_data.mint);

    // Step 4: Read cohort data from database
    println!("\n📦 Reading cohort data from database...");
    let cohort_data = read_cohort_data(&campaign_db_in)?;
    println!("✅ Found {} cohorts", cohort_data.len());
    for cohort in &cohort_data {
        println!("  📦 {}: {} vaults", cohort.name, cohort.vaults.len());
    }

    // Step 5: Deploy campaign PDA
    println!("\n🏗️  Deploying campaign PDA...");
    deploy_campaign_pda(&rpc_client, &admin_keypair, &campaign_data)?;

    // Step 6: Deploy all cohort PDAs
    println!("\n🏗️  Deploying cohort PDAs...");
    for cohort in &cohort_data {
        deploy_cohort_pda(&rpc_client, &admin_keypair, &campaign_data, cohort)?;
    }

    // Step 7: Update database with deployment timestamps
    println!("\n💾 Updating database with deployment status...");
    update_deployment_status(&campaign_db_in)?;

    println!("\n🎉 Campaign deployment completed successfully!");
    println!("📊 Summary:");
    println!("  - Campaign PDA deployed");
    println!("  - {} cohort PDAs deployed", cohort_data.len());
    println!("  - Database updated with deployment timestamps");

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

fn read_cohort_data(db_path: &PathBuf) -> CliResult<Vec<CohortData>> {
    let conn = Connection::open(db_path)?;

    // First get cohort basic data
    let mut stmt =
        conn.prepare("SELECT cohort_name, merkle_root, amount_per_entitlement FROM cohorts")?;
    let cohort_rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let merkle_root_hex: String = row.get(1)?;
        let amount_per_entitlement: u64 = row.get(2)?;

        Ok((name, merkle_root_hex, amount_per_entitlement))
    })?;

    let mut cohorts = Vec::new();

    for row in cohort_rows {
        let (name, merkle_root_hex, amount_per_entitlement) = row?;

        let merkle_root_bytes = hex::decode(merkle_root_hex)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid merkle root hex: {}", e)))?;
        let merkle_root: [u8; 32] = merkle_root_bytes
            .try_into()
            .map_err(|_| CliError::InvalidConfig("Merkle root must be 32 bytes".to_string()))?;

        // Get vaults for this cohort
        let mut vault_stmt = conn.prepare(
            "SELECT vault_pubkey FROM vaults WHERE cohort_name = ? ORDER BY vault_index",
        )?;
        let vault_rows = vault_stmt.query_map([&name], |row| {
            let vault_str: String = row.get(0)?;
            Ok(vault_str)
        })?;

        let mut vaults = Vec::new();
        for vault_row in vault_rows {
            let vault_str = vault_row?;
            let vault_pubkey = Pubkey::from_str(&vault_str)
                .map_err(|e| CliError::InvalidConfig(format!("Invalid vault pubkey: {}", e)))?;
            vaults.push(vault_pubkey);
        }

        cohorts.push(CohortData {
            name,
            merkle_root,
            amount_per_entitlement,
            vaults,
        });
    }

    Ok(cohorts)
}

fn deploy_campaign_pda(
    rpc_client: &RpcClient,
    admin_keypair: &dyn Signer,
    campaign_data: &CampaignData,
) -> CliResult<()> {
    let (campaign_address, _) =
        find_campaign_address(&campaign_data.admin, &campaign_data.fingerprint);

    println!("  📍 Campaign PDA: {}", campaign_address);

    // Check if already deployed
    if let Ok(account) = rpc_client.get_account(&campaign_address) {
        if !account.data.is_empty() {
            println!("  ⚠️  Campaign PDA already exists, skipping...");
            return Ok(());
        }
    }

    // Build initialize campaign instruction
    let (initialize_campaign_ix, _, _) = build_initialize_campaign_ix(
        campaign_data.admin,
        campaign_address,
        campaign_data.fingerprint,
        campaign_data.mint,
    )
    .map_err(|e| CliError::InvalidConfig(format!("Failed to build campaign instruction: {}", e)))?;

    // Create and send transaction with enhanced retry logic
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?;

    let transaction = Transaction::new_signed_with_payer(
        &[initialize_campaign_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    // Use enhanced transaction sending with spinner and retry logic
    let config = RpcSendTransactionConfig {
        skip_preflight: false, // Keep preflight checks for better error handling
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        encoding: None,
        max_retries: Some(5), // Allow up to 5 retries
        min_context_slot: None,
    };

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner_and_config(
            &transaction,
            CommitmentConfig::confirmed(),
            config,
        )
        .map_err(|e| CliError::InvalidConfig(format!("Failed to deploy campaign: {}", e)))?;

    println!("  ✅ Campaign PDA deployed! Signature: {}", signature);
    Ok(())
}

fn deploy_cohort_pda(
    rpc_client: &RpcClient,
    admin_keypair: &dyn Signer,
    campaign_data: &CampaignData,
    cohort_data: &CohortData,
) -> CliResult<()> {
    let (campaign_address, _) =
        find_campaign_address(&campaign_data.admin, &campaign_data.fingerprint);
    let (cohort_address, _) = find_cohort_v0_address(&campaign_address, &cohort_data.merkle_root);

    println!("  📦 Deploying cohort: {}", cohort_data.name);
    println!("    📍 Cohort PDA: {}", cohort_address);

    // Check if already deployed
    if let Ok(account) = rpc_client.get_account(&cohort_address) {
        if !account.data.is_empty() {
            println!("    ⚠️  Cohort PDA already exists, skipping...");
            return Ok(());
        }
    }

    // Build initialize cohort instruction
    let (initialize_cohort_ix, _, _) = build_initialize_cohort_ix(
        campaign_data.admin,
        campaign_address,
        campaign_data.fingerprint,
        cohort_address,
        cohort_data.merkle_root,
        cohort_data.amount_per_entitlement,
        cohort_data.vaults.clone(),
    )
    .map_err(|e| CliError::InvalidConfig(format!("Failed to build cohort instruction: {}", e)))?;

    // Create and send transaction with enhanced retry logic
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?;

    let transaction = Transaction::new_signed_with_payer(
        &[initialize_cohort_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    // Use enhanced transaction sending with spinner and retry logic
    let config = RpcSendTransactionConfig {
        skip_preflight: false, // Keep preflight checks for better error handling
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        encoding: None,
        max_retries: Some(5), // Allow up to 5 retries
        min_context_slot: None,
    };

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner_and_config(
            &transaction,
            CommitmentConfig::confirmed(),
            config,
        )
        .map_err(|e| CliError::InvalidConfig(format!("Failed to deploy cohort: {}", e)))?;

    println!("    ✅ Cohort PDA deployed! Signature: {}", signature);
    Ok(())
}

fn update_deployment_status(db_path: &PathBuf) -> CliResult<()> {
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().timestamp();

    // Update campaign deployment timestamp
    conn.execute("UPDATE campaign SET deployed_at = ?", [now])?;

    // Update all cohorts deployment timestamp
    conn.execute("UPDATE cohorts SET deployed_at = ?", [now])?;

    Ok(())
}

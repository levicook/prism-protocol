/*!
# Deploy Campaign Command

This command performs a comprehensive deployment of a Prism Protocol campaign on-chain.
It takes a compiled campaign database (from `compile-campaign`) and deploys everything
needed for claimants to start claiming tokens.

## Complete Deployment Process

The deploy-campaign command performs these steps in order:

### 1. Pre-flight Checks
- ✅ Validate admin keypair can be read
- ✅ Connect to Solana RPC
- ✅ Read campaign data from database
- ✅ Check admin has sufficient SOL for rent costs
- ✅ Check admin has sufficient tokens to fund all vaults (transfer, not mint)
- ✅ Validate admin has token accounts with sufficient balance

### 2. Campaign Initialization
- ✅ Deploy campaign PDA (if not already deployed)
- ✅ Verify campaign starts inactive/paused

### 3. Cohort Initialization
- ✅ Deploy all cohort PDAs (if not already deployed)
- ✅ Verify cohorts are properly linked to campaign

### 4. Vault Creation & Funding
- ✅ Create all vault token accounts (if not already created)
- ✅ Fund all vaults with required tokens
- ✅ Verify vault balances match requirements

### 5. Campaign Activation
- ✅ Enable/activate the campaign after everything is funded
- ✅ Final verification that campaign is ready for claims

## Error Handling
- Should be idempotent - can be run multiple times safely
- Should provide clear progress indicators
- Should fail fast on critical errors (insufficient funds, etc.)
- Should provide detailed error messages for troubleshooting

## Database Updates
- Track deployment status and signatures for all components
- Update funding status and transaction signatures
- Record final activation status and timestamp
*/

use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_sdk::{
    instruction_builders::{
        build_create_vault_ix, build_initialize_campaign_ix, build_initialize_cohort_ix,
        build_set_campaign_active_status_ix,
    },
    AddressFinder,
};
use rusqlite::Connection;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{self, state::Mint};
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
    vault_count: usize,
}

#[derive(Debug)]
struct VaultData {
    cohort_name: String,
    vault_index: usize,
    required_tokens: u64,
}

/// Fetch the mint account and get the number of decimals
fn get_mint_decimals(rpc_client: &RpcClient, mint: &Pubkey) -> CliResult<u8> {
    let account_data = rpc_client.get_account_data(mint).map_err(|e| {
        CliError::InvalidConfig(format!("Failed to fetch mint account {}: {}", mint, e))
    })?;

    let mint_info = Mint::unpack(&account_data).map_err(|e| {
        CliError::InvalidConfig(format!("Failed to parse mint account {}: {}", mint, e))
    })?;

    Ok(mint_info.decimals)
}

/// Convert base units to human-readable format using actual mint decimals
fn format_token_amount(base_units: u64, decimals: u8) -> String {
    let divisor = 10_u64.pow(decimals as u32);
    let whole_tokens = base_units / divisor;
    let fractional_units = base_units % divisor;

    if fractional_units == 0 {
        format!("{}", whole_tokens)
    } else {
        // Format with trailing zeros removed
        let fractional_str = format!("{:0width$}", fractional_units, width = decimals as usize);
        let trimmed = fractional_str.trim_end_matches('0');
        if trimmed.is_empty() {
            format!("{}", whole_tokens)
        } else {
            format!("{}.{}", whole_tokens, trimmed)
        }
    }
}

/// Calculate actual tokens needed for funding (excluding already-funded vaults)
fn calculate_actual_tokens_needed(
    rpc_client: &RpcClient,
    campaign_data: &CampaignData,
    cohort_data: &[CohortData],
    vault_requirements: &[VaultData],
) -> CliResult<u64> {
    let address_finder = AddressFinder::default();

    let mut actual_tokens_needed = 0u64;

    for vault_req in vault_requirements {
        // Find the corresponding cohort
        let cohort = cohort_data
            .iter()
            .find(|c| c.name == vault_req.cohort_name)
            .ok_or_else(|| {
                CliError::InvalidConfig(format!("Cohort {} not found", vault_req.cohort_name))
            })?;

        // Derive vault address
        let (campaign_address, _) = address_finder
            .find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

        let (cohort_address, _) =
            address_finder.find_cohort_v0_address(&campaign_address, &cohort.merkle_root);

        let (vault_address, _) =
            address_finder.find_vault_v0_address(&cohort_address, vault_req.vault_index as u8);

        // Check current vault balance
        let current_balance = get_vault_token_balance(rpc_client, &vault_address).unwrap_or(0);

        // Only count tokens still needed
        if current_balance < vault_req.required_tokens {
            actual_tokens_needed += vault_req.required_tokens - current_balance;
        }
    }

    Ok(actual_tokens_needed)
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

    // Step 5: Read vault funding requirements
    println!("\n💰 Reading vault funding requirements...");
    let vault_requirements = read_vault_requirements(&campaign_db_in)?;
    let total_tokens_needed: u64 = vault_requirements.iter().map(|v| v.required_tokens).sum();

    // Fetch actual mint decimals from blockchain
    let mint_decimals = get_mint_decimals(&rpc_client, &campaign_data.mint)?;

    println!(
        "✅ Found {} vaults requiring {} base units ({} tokens)",
        vault_requirements.len(),
        total_tokens_needed,
        format_token_amount(total_tokens_needed, mint_decimals)
    );

    // Show per-cohort breakdown
    let mut cohort_totals: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    for vault in &vault_requirements {
        *cohort_totals.entry(vault.cohort_name.clone()).or_insert(0) += vault.required_tokens;
    }

    println!("📊 Funding breakdown by cohort:");
    for (cohort_name, tokens) in &cohort_totals {
        println!(
            "  📦 {}: {} base units ({} tokens)",
            cohort_name,
            tokens,
            format_token_amount(*tokens, mint_decimals)
        );
    }

    // Show WSOL funding instructions if using WSOL
    if campaign_data.mint.to_string() == "So11111111111111111111111111111111111111112" {
        let human_amount = format_token_amount(total_tokens_needed, mint_decimals);
        let buffer_amount = format_token_amount(total_tokens_needed + 1_000_000, mint_decimals); // 0.001 SOL buffer

        println!("\n💡 WSOL Funding Instructions:");
        println!("   To wrap SOL for funding this campaign:");
        println!("   ");
        println!("   1. Check for existing WSOL account:");
        println!("      spl-token accounts");
        println!("   ");
        println!("   2. If WSOL account exists, unwrap it first:");
        println!("      spl-token unwrap <WSOL-ACCOUNT-ADDRESS>");
        println!("   ");
        println!("   3. Wrap the required amount:");
        println!("      spl-token wrap {}", buffer_amount);
        println!("   ");
        println!("   (Required: {} + small buffer for fees)", human_amount);
    }

    // Step 6: Pre-flight checks
    println!("\n🔍 Performing pre-flight checks...");

    // Calculate actual tokens needed (accounting for already-funded vaults)
    let actual_tokens_needed = calculate_actual_tokens_needed(
        &rpc_client,
        &campaign_data,
        &cohort_data,
        &vault_requirements,
    )?;

    if actual_tokens_needed < total_tokens_needed {
        println!("💡 Some vaults are already funded:");
        println!(
            "   Total required: {} base units ({} tokens)",
            total_tokens_needed,
            format_token_amount(total_tokens_needed, mint_decimals)
        );
        println!(
            "   Actually needed: {} base units ({} tokens)",
            actual_tokens_needed,
            format_token_amount(actual_tokens_needed, mint_decimals)
        );
    }

    perform_preflight_checks(
        &rpc_client,
        &admin_keypair,
        &campaign_data,
        total_tokens_needed,
    )?;

    // Step 7: Deploy campaign PDA
    println!("\n🏗️  Deploying campaign PDA...");
    let campaign_signature =
        deploy_campaign_pda(&rpc_client, &admin_keypair, &campaign_data, &campaign_db_in)?;

    // Step 8: Deploy all cohort PDAs and their vaults
    println!("\n🏗️  Deploying cohort PDAs and vaults...");
    println!("📊 Progress: 0/{} cohorts deployed", cohort_data.len());
    let mut cohort_signatures = Vec::new();

    for (index, cohort) in cohort_data.iter().enumerate() {
        // Deploy cohort PDA
        let cohort_signature = deploy_cohort_pda(
            &rpc_client,
            &admin_keypair,
            &campaign_data,
            cohort,
            &campaign_db_in,
        )?;
        if !cohort_signature.is_empty() {
            cohort_signatures.push((cohort.name.clone(), cohort_signature));
        }

        // Deploy and fund vaults for this cohort
        println!(
            "      🏗️  Creating and funding vaults for cohort {}...",
            cohort.name
        );
        let vault_signatures = deploy_and_fund_cohort_vaults(
            &rpc_client,
            &admin_keypair,
            &campaign_data,
            cohort,
            &campaign_db_in,
        )?;

        if !vault_signatures.is_empty() {
            println!(
                "      ✅ Created and funded {} vaults for cohort {}",
                vault_signatures.len(),
                cohort.name
            );
        } else {
            println!(
                "      ⚠️  All vaults for cohort {} already existed and were funded",
                cohort.name
            );
        }

        println!(
            "📊 Progress: {}/{} cohorts processed",
            index + 1,
            cohort_data.len()
        );
    }

    // Step 9: Activate campaign
    println!("\n🎯 Activating campaign...");
    activate_campaign(&rpc_client, &admin_keypair, &campaign_data, &campaign_db_in)?;

    // Step 10: Final verification
    println!("\n✅ Performing final verification...");
    verify_deployment(
        &rpc_client,
        &campaign_data,
        &cohort_data,
        &vault_requirements,
    )?;

    println!("\n🎉 Campaign deployment completed successfully!");
    println!("📊 Summary:");
    if !campaign_signature.is_empty() {
        println!("  - Campaign PDA deployed: {}", campaign_signature);
    } else {
        println!("  - Campaign PDA already existed (skipped)");
    }

    if !cohort_signatures.is_empty() {
        println!("  - {} cohort PDAs deployed:", cohort_signatures.len());
        for (cohort_name, signature) in &cohort_signatures {
            println!("    📦 {}: {}", cohort_name, signature);
        }
    } else {
        println!("  - All cohort PDAs already existed (skipped)");
    }

    println!("  - {} vaults created and funded", vault_requirements.len());
    println!("  - {} total tokens distributed", total_tokens_needed);
    println!("  - Campaign activated and ready for claims");
    println!("  - Database updated with deployment status");

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

        let vault_count = vaults.len();
        cohorts.push(CohortData {
            name,
            merkle_root,
            amount_per_entitlement,
            vaults,
            vault_count,
        });
    }

    Ok(cohorts)
}

fn read_vault_requirements(db_path: &PathBuf) -> CliResult<Vec<VaultData>> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT cohort_name, vault_index, required_tokens 
         FROM vaults ORDER BY cohort_name, vault_index",
    )?;

    let vault_rows = stmt.query_map([], |row| {
        let cohort_name: String = row.get(0)?;
        let vault_index: i64 = row.get(1)?;
        let required_tokens: u64 = row.get(2)?;

        Ok((cohort_name, vault_index, required_tokens))
    })?;

    let mut vault_requirements = Vec::new();
    for row in vault_rows {
        let (cohort_name, vault_index, required_tokens) = row?;

        vault_requirements.push(VaultData {
            cohort_name,
            vault_index: vault_index as usize,
            required_tokens,
        });
    }

    Ok(vault_requirements)
}

fn deploy_campaign_pda(
    rpc_client: &RpcClient,
    admin_keypair: &dyn Signer,
    campaign_data: &CampaignData,
    db_path: &PathBuf,
) -> CliResult<String> {
    let address_finder = AddressFinder::default();
    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

    println!("  📍 Campaign PDA: {}", campaign_address);

    // Check if already deployed
    if let Ok(account) = rpc_client.get_account(&campaign_address) {
        if !account.data.is_empty() {
            println!("  ⚠️  Campaign PDA already exists, skipping...");
            // Return empty string to indicate no new deployment
            return Ok(String::new());
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
    println!("  🔄 Getting recent blockhash...");
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?;

    let transaction = Transaction::new_signed_with_payer(
        &[initialize_campaign_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    println!("  📤 Sending campaign initialization transaction...");

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

    println!("  💾 Updating database with deployment status...");
    update_campaign_deployment_status(db_path, &signature.to_string())?;
    println!("  ✅ Database updated with campaign deployment status");

    Ok(signature.to_string())
}

fn deploy_cohort_pda(
    rpc_client: &RpcClient,
    admin_keypair: &dyn Signer,
    campaign_data: &CampaignData,
    cohort_data: &CohortData,
    db_path: &PathBuf,
) -> CliResult<String> {
    let address_finder = AddressFinder::default();

    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

    let (cohort_address, _) =
        address_finder.find_cohort_v0_address(&campaign_address, &cohort_data.merkle_root);

    println!("  📦 Deploying cohort: {}", cohort_data.name);
    println!("    📍 Cohort PDA: {}", cohort_address);

    // Check if already deployed
    if let Ok(account) = rpc_client.get_account(&cohort_address) {
        if !account.data.is_empty() {
            println!("    ⚠️  Cohort PDA already exists, skipping...");
            // Return empty string to indicate no new deployment
            return Ok(String::new());
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
        cohort_data.vault_count as u8,
    )
    .map_err(|e| CliError::InvalidConfig(format!("Failed to build cohort instruction: {}", e)))?;

    // Create and send transaction with enhanced retry logic
    println!("    🔄 Getting recent blockhash...");
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?;

    let transaction = Transaction::new_signed_with_payer(
        &[initialize_cohort_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    println!("    📤 Sending cohort initialization transaction...");

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

    // Immediately update database with cohort deployment status and signature
    println!("    💾 Updating database with deployment status...");
    update_cohort_deployment_status(db_path, &cohort_data.name, &signature.to_string())?;
    println!("    ✅ Database updated with cohort deployment status");

    Ok(signature.to_string())
}

fn update_campaign_deployment_status(db_path: &PathBuf, signature: &str) -> CliResult<()> {
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().timestamp();

    // Update campaign deployment timestamp and signature
    conn.execute(
        "UPDATE campaign SET deployed_at = ?, deployed_signature = ?",
        (now, signature),
    )?;

    Ok(())
}

fn update_cohort_deployment_status(
    db_path: &PathBuf,
    cohort_name: &str,
    signature: &str,
) -> CliResult<()> {
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().timestamp();

    // Update cohort deployment timestamp and signature
    conn.execute(
        "UPDATE cohorts SET deployed_at = ?, deployed_signature = ? WHERE cohort_name = ?",
        (now, signature, cohort_name),
    )?;

    Ok(())
}

fn deploy_and_fund_cohort_vaults(
    rpc_client: &RpcClient,
    admin_keypair: &dyn Signer,
    campaign_data: &CampaignData,
    cohort_data: &CohortData,
    db_path: &PathBuf,
) -> CliResult<Vec<String>> {
    let address_finder = AddressFinder::default();

    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

    let (cohort_address, _) =
        address_finder.find_cohort_v0_address(&campaign_address, &cohort_data.merkle_root);

    let mut vault_signatures = Vec::new();

    // Get vault requirements from database for this cohort
    let vault_requirements = read_vault_requirements_for_cohort(db_path, &cohort_data.name)?;

    // Process each vault: create first, then fund
    for vault_req in vault_requirements {
        let vault_index = vault_req.vault_index as u8;

        let (vault_address, _) = address_finder.find_vault_v0_address(&cohort_address, vault_index);

        println!(
            "        🏗️  Processing vault {} at {}",
            vault_index, vault_address
        );

        // Step 1: Create vault if it doesn't exist
        let creation_signature = create_vault_if_needed(
            rpc_client,
            admin_keypair,
            campaign_data,
            cohort_data,
            &vault_address,
            vault_index,
            db_path,
        )?;

        if !creation_signature.is_empty() {
            vault_signatures.push(creation_signature.clone());
        }

        // Step 2: Fund vault if it needs tokens
        if vault_req.required_tokens > 0 {
            fund_vault_if_needed(
                rpc_client,
                admin_keypair,
                &campaign_data.mint,
                &vault_address,
                vault_req.required_tokens,
                db_path,
                &cohort_data.name,
                vault_req.vault_index,
            )?;
        }
    }

    Ok(vault_signatures)
}

fn create_vault_if_needed(
    rpc_client: &RpcClient,
    admin_keypair: &dyn Signer,
    campaign_data: &CampaignData,
    cohort_data: &CohortData,
    vault_address: &Pubkey,
    vault_index: u8,
    db_path: &PathBuf,
) -> CliResult<String> {
    // Check if vault already exists
    let vault_exists = if let Ok(account) = rpc_client.get_account(vault_address) {
        account.lamports > 0
    } else {
        false
    };

    if vault_exists {
        println!(
            "        ⚠️  Vault {} already exists, skipping creation...",
            vault_index
        );
        return Ok(String::new());
    }

    println!("        📤 Creating vault {}...", vault_index);

    let address_finder = AddressFinder::default();

    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

    let (cohort_address, _) =
        address_finder.find_cohort_v0_address(&campaign_address, &cohort_data.merkle_root);

    // Build create vault instruction
    let (create_vault_ix, _, _) = build_create_vault_ix(
        campaign_data.admin,
        campaign_address,
        cohort_address,
        campaign_data.mint,
        *vault_address,
        campaign_data.fingerprint,
        cohort_data.merkle_root,
        vault_index,
    )
    .map_err(|e| {
        CliError::InvalidConfig(format!("Failed to build create vault instruction: {}", e))
    })?;

    // Create and send transaction
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?;

    let transaction = Transaction::new_signed_with_payer(
        &[create_vault_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    let config = RpcSendTransactionConfig {
        skip_preflight: false,
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        encoding: None,
        max_retries: Some(5),
        min_context_slot: None,
    };

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner_and_config(
            &transaction,
            CommitmentConfig::confirmed(),
            config,
        )
        .map_err(|e| {
            CliError::InvalidConfig(format!("Failed to create vault {}: {}", vault_index, e))
        })?;

    println!(
        "        ✅ Vault {} created! Signature: {}",
        vault_index, signature
    );

    // Update database with vault creation status
    update_vault_creation_status(
        db_path,
        &cohort_data.name,
        vault_index as usize,
        vault_address,
        &signature.to_string(),
    )?;

    Ok(signature.to_string())
}

fn fund_vault_if_needed(
    rpc_client: &RpcClient,
    admin_keypair: &dyn Signer,
    mint: &Pubkey,
    vault_address: &Pubkey,
    required_tokens: u64,
    db_path: &PathBuf,
    cohort_name: &str,
    vault_index: usize,
) -> CliResult<()> {
    // Check current vault balance
    let current_balance = get_vault_token_balance(rpc_client, vault_address)?;

    if current_balance >= required_tokens {
        println!(
            "        ✅ Vault {} already sufficiently funded ({} tokens)",
            vault_index, current_balance
        );
        return Ok(());
    }

    let tokens_needed = required_tokens - current_balance;
    println!(
        "        💰 Funding vault {} with {} tokens (current: {}, needed: {})...",
        vault_index, tokens_needed, current_balance, required_tokens
    );

    // Transfer tokens from admin's token account to vault
    let admin_token_account = get_associated_token_address(&admin_keypair.pubkey(), mint);

    let transfer_ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &admin_token_account,
        vault_address,
        &admin_keypair.pubkey(),
        &[],
        tokens_needed,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    let signature = rpc_client.send_and_confirm_transaction_with_spinner(&transaction)?;
    println!(
        "        ✅ Vault {} funded with {} tokens! Signature: {}",
        vault_index, tokens_needed, signature
    );

    // Update database with vault funding status
    update_vault_funding_status(db_path, cohort_name, vault_index, &signature.to_string())?;

    Ok(())
}

fn update_vault_creation_status(
    db_path: &PathBuf,
    cohort_name: &str,
    vault_index: usize,
    vault_address: &Pubkey,
    signature: &str,
) -> CliResult<()> {
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().timestamp();

    // Update vault with creation details
    conn.execute(
        "UPDATE vaults SET vault_pubkey = ?, created_at = ?, created_by_tx = ? 
         WHERE cohort_name = ? AND vault_index = ?",
        (
            vault_address.to_string(),
            now,
            signature,
            cohort_name,
            vault_index,
        ),
    )?;

    Ok(())
}

fn update_vault_funding_status(
    db_path: &PathBuf,
    cohort_name: &str,
    vault_index: usize,
    signature: &str,
) -> CliResult<()> {
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().timestamp();

    // Update vault with funding details
    conn.execute(
        "UPDATE vaults SET funded_at = ?, funded_by_tx = ? 
         WHERE cohort_name = ? AND vault_index = ?",
        (now, signature, cohort_name, vault_index),
    )?;

    Ok(())
}

fn perform_preflight_checks(
    rpc_client: &RpcClient,
    admin_keypair: &dyn Signer,
    campaign_data: &CampaignData,
    total_tokens_needed: u64,
) -> CliResult<()> {
    let admin_pubkey = admin_keypair.pubkey();

    // Check 1: Admin SOL balance for rent costs
    println!("  💰 Checking admin SOL balance...");
    let admin_balance = rpc_client
        .get_balance(&admin_pubkey)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get admin balance: {}", e)))?;

    // Rough estimate: Campaign + Cohorts + Vaults rent costs
    // This is conservative - actual costs may be lower
    let estimated_rent_cost = 10_000_000; // ~0.01 SOL buffer for rent

    if admin_balance < estimated_rent_cost {
        return Err(CliError::InvalidConfig(format!(
            "Insufficient SOL balance. Admin has {} lamports, need at least {} for rent costs",
            admin_balance, estimated_rent_cost
        )));
    }
    println!(
        "    ✅ Admin has {} SOL (sufficient for rent)",
        admin_balance as f64 / 1e9
    );

    // Check 2: Token balance for transfers
    if total_tokens_needed > 0 {
        println!("  🪙 Checking admin token balance for transfers...");

        // For all tokens (including wrapped SOL), check admin's token account balance
        let admin_token_account = get_associated_token_address(&admin_pubkey, &campaign_data.mint);

        match rpc_client.get_account(&admin_token_account) {
            Ok(token_account) => {
                // Parse token account to check balance
                if token_account.data.len() >= 64 {
                    // Token account amount is at bytes 64-72 (u64 little endian)
                    let amount_bytes: [u8; 8] =
                        token_account.data[64..72].try_into().map_err(|_| {
                            CliError::InvalidConfig("Invalid token account data".to_string())
                        })?;
                    let current_balance = u64::from_le_bytes(amount_bytes);

                    if current_balance < total_tokens_needed {
                        return Err(CliError::InvalidConfig(format!(
                            "Insufficient token balance. Admin has {} tokens, need {} for vault funding",
                            current_balance, total_tokens_needed
                        )));
                    }
                    println!(
                        "    ✅ Admin has {} tokens (sufficient for transfers)",
                        current_balance
                    );
                } else {
                    return Err(CliError::InvalidConfig(
                        "Invalid token account data length".to_string(),
                    ));
                }
            }
            Err(_) => {
                return Err(CliError::InvalidConfig(format!(
                    "Admin token account {} not found. Admin must have tokens to transfer to vaults",
                    admin_token_account
                )));
            }
        }
    }

    // Check 3: RPC connection stability
    println!("  🌐 Verifying RPC connection...");
    let _slot = rpc_client
        .get_slot()
        .map_err(|e| CliError::InvalidConfig(format!("RPC connection unstable: {}", e)))?;
    println!("    ✅ RPC connection stable");

    println!("✅ All pre-flight checks passed");
    Ok(())
}

fn read_vault_requirements_for_cohort(
    db_path: &PathBuf,
    cohort_name: &str,
) -> CliResult<Vec<VaultData>> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT cohort_name, vault_index, required_tokens 
         FROM vaults WHERE cohort_name = ? ORDER BY vault_index",
    )?;

    let vault_rows = stmt.query_map([cohort_name], |row| {
        let cohort_name: String = row.get(0)?;
        let vault_index: i64 = row.get(1)?;
        let required_tokens: u64 = row.get(2)?;

        Ok((cohort_name, vault_index, required_tokens))
    })?;

    let mut vault_requirements = Vec::new();
    for row in vault_rows {
        let (cohort_name, vault_index, required_tokens) = row?;

        vault_requirements.push(VaultData {
            cohort_name,
            vault_index: vault_index as usize,
            required_tokens,
        });
    }

    Ok(vault_requirements)
}

fn get_vault_token_balance(rpc_client: &RpcClient, vault_address: &Pubkey) -> CliResult<u64> {
    match rpc_client.get_account(vault_address) {
        Ok(account) => {
            if account.data.len() >= 72 {
                // Token account amount is at bytes 64-72 (u64 little endian)
                let amount_bytes: [u8; 8] = account.data[64..72].try_into().map_err(|_| {
                    CliError::InvalidConfig("Invalid vault token account data".to_string())
                })?;
                Ok(u64::from_le_bytes(amount_bytes))
            } else {
                Ok(0) // Account exists but not initialized as token account
            }
        }
        Err(_) => Ok(0), // Account doesn't exist
    }
}

fn activate_campaign(
    rpc_client: &RpcClient,
    admin_keypair: &dyn Signer,
    campaign_data: &CampaignData,
    db_path: &PathBuf,
) -> CliResult<()> {
    let address_finder = AddressFinder::default();

    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

    // Check if campaign exists
    match rpc_client.get_account(&campaign_address) {
        Ok(_account) => {
            println!("  🎯 Campaign PDA found, activating campaign...");

            // Build activation instruction
            let (activate_ix, _, _) = build_set_campaign_active_status_ix(
                campaign_data.admin,
                campaign_address,
                campaign_data.fingerprint,
                true, // Set to active
            )
            .map_err(|e| {
                CliError::InvalidConfig(format!("Failed to build activation instruction: {}", e))
            })?;

            // Send activation transaction
            let recent_blockhash = rpc_client
                .get_latest_blockhash()
                .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?;

            let transaction = Transaction::new_signed_with_payer(
                &[activate_ix],
                Some(&admin_keypair.pubkey()),
                &[admin_keypair],
                recent_blockhash,
            );

            let config = RpcSendTransactionConfig {
                skip_preflight: false,
                preflight_commitment: Some(CommitmentLevel::Confirmed),
                encoding: None,
                max_retries: Some(5),
                min_context_slot: None,
            };

            let signature = rpc_client
                .send_and_confirm_transaction_with_spinner_and_config(
                    &transaction,
                    CommitmentConfig::confirmed(),
                    config,
                )
                .map_err(|e| {
                    CliError::InvalidConfig(format!("Failed to activate campaign: {}", e))
                })?;

            println!("  ✅ Campaign activated! Signature: {}", signature);

            // Update database with activation status
            update_campaign_activation_status(db_path, &signature.to_string())?;
        }
        Err(_) => {
            return Err(CliError::InvalidConfig(
                "Campaign PDA not found - deployment may have failed".to_string(),
            ));
        }
    }

    Ok(())
}

fn verify_deployment(
    rpc_client: &RpcClient,
    campaign_data: &CampaignData,
    cohort_data: &[CohortData],
    vault_requirements: &[VaultData],
) -> CliResult<()> {
    println!("  🔍 Verifying campaign deployment...");

    // Verify campaign PDA exists
    let address_finder = AddressFinder::default();

    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

    match rpc_client.get_account(&campaign_address) {
        Ok(_) => println!("    ✅ Campaign PDA verified"),
        Err(_) => {
            return Err(CliError::InvalidConfig(
                "Campaign PDA verification failed".to_string(),
            ))
        }
    }

    // Verify all cohort PDAs exist
    for cohort in cohort_data {
        let (cohort_address, _) =
            address_finder.find_cohort_v0_address(&campaign_address, &cohort.merkle_root);

        match rpc_client.get_account(&cohort_address) {
            Ok(_) => println!("    ✅ Cohort {} PDA verified", cohort.name),
            Err(_) => {
                return Err(CliError::InvalidConfig(format!(
                    "Cohort {} PDA verification failed",
                    cohort.name
                )))
            }
        }
    }

    // Verify all vaults exist and are funded
    let mut total_verified_tokens = 0u64;
    for vault_req in vault_requirements {
        let cohort = cohort_data
            .iter()
            .find(|c| c.name == vault_req.cohort_name)
            .ok_or_else(|| {
                CliError::InvalidConfig(format!("Cohort {} not found", vault_req.cohort_name))
            })?;

        let (campaign_address, _) = address_finder
            .find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

        let (cohort_address, _) =
            address_finder.find_cohort_v0_address(&campaign_address, &cohort.merkle_root);

        let (vault_address, _) =
            address_finder.find_vault_v0_address(&cohort_address, vault_req.vault_index as u8);

        match rpc_client.get_account(&vault_address) {
            Ok(_) => {
                let balance = get_vault_token_balance(rpc_client, &vault_address)?;
                if balance >= vault_req.required_tokens {
                    println!(
                        "    ✅ Vault {}/{} verified ({} tokens)",
                        vault_req.cohort_name, vault_req.vault_index, balance
                    );
                    total_verified_tokens += balance;
                } else {
                    return Err(CliError::InvalidConfig(format!(
                        "Vault {}/{} underfunded: has {}, needs {}",
                        vault_req.cohort_name,
                        vault_req.vault_index,
                        balance,
                        vault_req.required_tokens
                    )));
                }
            }
            Err(_) => {
                return Err(CliError::InvalidConfig(format!(
                    "Vault {}/{} verification failed",
                    vault_req.cohort_name, vault_req.vault_index
                )))
            }
        }
    }

    println!("  ✅ All components verified successfully");
    println!("  📊 Total tokens in vaults: {}", total_verified_tokens);

    Ok(())
}

fn update_campaign_activation_status(db_path: &PathBuf, signature: &str) -> CliResult<()> {
    let conn = Connection::open(db_path)?;
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "UPDATE campaign SET activated_at = ?, activation_signature = ?",
        [now.to_string(), signature.to_string()],
    )?;

    Ok(())
}

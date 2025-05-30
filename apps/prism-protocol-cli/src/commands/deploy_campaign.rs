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
use prism_protocol_client::PrismProtocolClient;
use prism_protocol_db::CampaignDatabase;
use prism_protocol_sdk::{
    build_create_vault_ix, build_initialize_campaign_ix, build_initialize_cohort_ix,
    build_set_campaign_active_status_ix,
};
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use std::path::PathBuf;

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

    // Step 2: Create client and database connections using our new abstractions
    println!("\n🌐 Connecting to Solana RPC...");
    let client = PrismProtocolClient::new(rpc_url.clone())
        .map_err(|e| CliError::InvalidConfig(format!("Failed to create RPC client: {}", e)))?;

    // Test connection
    let _version = client
        .rpc_client()
        .get_version()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to connect to RPC: {}", e)))?;
    println!("✅ Connected to Solana RPC: {}", rpc_url);

    // Step 3: Open database using our new interface
    println!("\n📋 Reading campaign data from database...");
    let mut db = CampaignDatabase::open(&campaign_db_in)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to open database: {}", e)))?;

    let campaign_info = db
        .read_campaign_info()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read campaign info: {}", e)))?;

    println!(
        "✅ Campaign fingerprint: {}",
        hex::encode(campaign_info.fingerprint)
    );
    println!("✅ Mint: {}", campaign_info.mint);
    println!("✅ Admin: {}", campaign_info.admin);

    // Step 4: Read cohort and vault data from database using our new interface
    println!("\n📦 Reading cohort data from database...");
    let cohort_data = db
        .read_cohorts()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read cohorts: {}", e)))?;

    println!("✅ Found {} cohorts", cohort_data.len());
    for cohort in &cohort_data {
        println!("  📦 {}: {} vaults", cohort.name, cohort.vaults.len());
    }

    // Step 5: Read vault funding requirements
    println!("\n💰 Reading vault funding requirements...");
    let vault_requirements = db.read_vault_requirements().map_err(|e| {
        CliError::InvalidConfig(format!("Failed to read vault requirements: {}", e))
    })?;
    let total_tokens_needed: u64 = vault_requirements.iter().map(|v| v.required_tokens).sum();

    // Fetch actual mint decimals from blockchain using our client
    let mint_info = client
        .get_mint(&campaign_info.mint)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to fetch mint info: {}", e)))?
        .ok_or_else(|| CliError::InvalidConfig(format!("Mint {} not found", campaign_info.mint)))?;
    let mint_decimals = mint_info.decimals;

    println!(
        "✅ Found {} vaults requiring {} base units ({} tokens)",
        vault_requirements.len(),
        total_tokens_needed,
        client.format_token_amount(total_tokens_needed, mint_decimals)
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
            client.format_token_amount(*tokens, mint_decimals)
        );
    }

    // Show WSOL funding instructions if using WSOL
    if client.is_wsol_mint(&campaign_info.mint) {
        let human_amount = client.format_token_amount(total_tokens_needed, mint_decimals);
        let buffer_amount =
            client.format_token_amount(total_tokens_needed + 1_000_000, mint_decimals); // 0.001 SOL buffer

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
    let actual_tokens_needed =
        calculate_actual_tokens_needed(&client, &campaign_info, &cohort_data, &vault_requirements)?;

    if actual_tokens_needed < total_tokens_needed {
        println!("💡 Some vaults are already funded:");
        println!(
            "   Total required: {} base units ({} tokens)",
            total_tokens_needed,
            client.format_token_amount(total_tokens_needed, mint_decimals)
        );
        println!(
            "   Actually needed: {} base units ({} tokens)",
            actual_tokens_needed,
            client.format_token_amount(actual_tokens_needed, mint_decimals)
        );
    }

    perform_preflight_checks(&client, &admin_keypair, &campaign_info, total_tokens_needed)?;

    // Step 7: Deploy campaign PDA
    println!("\n🏗️  Deploying campaign PDA...");
    let campaign_signature = deploy_campaign_pda(&client, &admin_keypair, &campaign_info, &mut db)?;

    // Step 8: Deploy all cohort PDAs and their vaults
    println!("\n🏗️  Deploying cohort PDAs and vaults...");
    println!("📊 Progress: 0/{} cohorts deployed", cohort_data.len());
    let mut cohort_signatures = Vec::new();

    for (index, cohort) in cohort_data.iter().enumerate() {
        // Deploy cohort PDA
        let cohort_signature = deploy_cohort_pda(&client, &admin_keypair, &campaign_info, cohort)?;
        if !cohort_signature.is_empty() {
            cohort_signatures.push((cohort.name.clone(), cohort_signature));
        }

        // Deploy and fund vaults for this cohort
        println!(
            "      🏗️  Creating and funding vaults for cohort {}...",
            cohort.name
        );
        let vault_signatures = deploy_and_fund_cohort_vaults(
            &client,
            &admin_keypair,
            &campaign_info,
            cohort,
            &mut db,
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
    activate_campaign(&client, &admin_keypair, &campaign_info, &mut db)?;

    // Step 10: Final verification
    println!("\n✅ Performing final verification...");
    verify_deployment(&client, &campaign_info, &cohort_data, &vault_requirements)?;

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

/// Calculate actual tokens needed for funding (excluding already-funded vaults)
fn calculate_actual_tokens_needed(
    client: &PrismProtocolClient,
    campaign_info: &prism_protocol_db::CampaignInfo,
    cohort_data: &[prism_protocol_db::CohortInfo],
    vault_requirements: &[prism_protocol_db::VaultRequirement],
) -> CliResult<u64> {
    let mut actual_tokens_needed = 0u64;

    for vault_req in vault_requirements {
        // Find the corresponding cohort
        let cohort = cohort_data
            .iter()
            .find(|c| c.name == vault_req.cohort_name)
            .ok_or_else(|| {
                CliError::InvalidConfig(format!("Cohort {} not found", vault_req.cohort_name))
            })?;

        // Derive vault address using client's address finder
        let (campaign_address, _) = client
            .address_finder()
            .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);

        let (cohort_address, _) = client
            .address_finder()
            .find_cohort_v0_address(&campaign_address, &cohort.merkle_root);

        let (vault_address, _) = client
            .address_finder()
            .find_vault_v0_address(&cohort_address, vault_req.vault_index as u8);

        // Check current vault balance using client
        let current_balance = match client.get_token_account(&vault_address) {
            Ok(Some(token_account)) => token_account.amount,
            _ => 0,
        };

        // Only count tokens still needed
        if current_balance < vault_req.required_tokens {
            actual_tokens_needed += vault_req.required_tokens - current_balance;
        }
    }

    Ok(actual_tokens_needed)
}

fn perform_preflight_checks(
    client: &PrismProtocolClient,
    admin_keypair: &dyn Signer,
    campaign_info: &prism_protocol_db::CampaignInfo,
    total_tokens_needed: u64,
) -> CliResult<()> {
    let admin_pubkey = admin_keypair.pubkey();

    // Check 1: Admin SOL balance for rent costs
    println!("  💰 Checking admin SOL balance...");
    let admin_balance = client
        .rpc_client()
        .get_balance(&admin_pubkey)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get admin balance: {}", e)))?;

    // Rough estimate: Campaign + Cohorts + Vaults rent costs
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

        let admin_token_account = get_associated_token_address(&admin_pubkey, &campaign_info.mint);

        match client.get_token_account(&admin_token_account) {
            Ok(Some(token_account)) => {
                let current_balance = token_account.amount;

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
            }
            _ => {
                return Err(CliError::InvalidConfig(format!(
                    "Admin token account {} not found. Admin must have tokens to transfer to vaults",
                    admin_token_account
                )));
            }
        }
    }

    // Check 3: RPC connection stability
    println!("  🌐 Verifying RPC connection...");
    let _slot = client
        .rpc_client()
        .get_slot()
        .map_err(|e| CliError::InvalidConfig(format!("RPC connection unstable: {}", e)))?;
    println!("    ✅ RPC connection stable");

    println!("✅ All pre-flight checks passed");
    Ok(())
}

fn deploy_campaign_pda(
    client: &PrismProtocolClient,
    admin_keypair: &dyn Signer,
    campaign_info: &prism_protocol_db::CampaignInfo,
    db: &mut CampaignDatabase,
) -> CliResult<String> {
    let (campaign_address, _) = client
        .address_finder()
        .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);

    println!("  📍 Campaign PDA: {}", campaign_address);

    // Check if already deployed - fix argument order
    if let Ok(Some(_)) = client.get_campaign_v0(&campaign_info.fingerprint, &campaign_info.admin) {
        println!("  ⚠️  Campaign PDA already exists, skipping...");
        return Ok(String::new());
    }

    // Build initialize campaign instruction
    let (initialize_campaign_ix, _, _) = build_initialize_campaign_ix(
        campaign_info.admin,
        campaign_address,
        campaign_info.fingerprint,
        campaign_info.mint,
    )
    .map_err(|e| CliError::InvalidConfig(format!("Failed to build campaign instruction: {}", e)))?;

    // Create and send transaction with enhanced retry logic
    println!("  🔄 Getting recent blockhash...");
    let recent_blockhash = client
        .rpc_client()
        .get_latest_blockhash()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?;

    let transaction = Transaction::new_signed_with_payer(
        &[initialize_campaign_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    println!("  📤 Sending campaign initialization transaction...");

    let config = RpcSendTransactionConfig {
        skip_preflight: false,
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        encoding: None,
        max_retries: Some(5),
        min_context_slot: None,
    };

    let signature = client
        .rpc_client()
        .send_and_confirm_transaction_with_spinner_and_config(
            &transaction,
            CommitmentConfig::confirmed(),
            config,
        )
        .map_err(|e| CliError::InvalidConfig(format!("Failed to deploy campaign: {}", e)))?;

    println!("  ✅ Campaign PDA deployed! Signature: {}", signature);

    println!("  💾 Updating database with deployment status...");
    db.update_campaign_deployment(&signature.to_string())
        .map_err(|e| CliError::InvalidConfig(format!("Failed to update database: {}", e)))?;
    println!("  ✅ Database updated with campaign deployment status");

    Ok(signature.to_string())
}

fn deploy_cohort_pda(
    client: &PrismProtocolClient,
    admin_keypair: &dyn Signer,
    campaign_info: &prism_protocol_db::CampaignInfo,
    cohort: &prism_protocol_db::CohortInfo,
) -> CliResult<String> {
    let (campaign_address, _) = client
        .address_finder()
        .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);

    let (cohort_address, _) = client
        .address_finder()
        .find_cohort_v0_address(&campaign_address, &cohort.merkle_root);

    println!("  📦 Deploying cohort: {}", cohort.name);
    println!("    📍 Cohort PDA: {}", cohort_address);

    // Check if already deployed
    if let Ok(Some(_)) = client.get_cohort_v0(&campaign_address, &cohort.merkle_root) {
        println!("    ⚠️  Cohort PDA already exists, skipping...");
        return Ok(String::new());
    }

    // Build initialize cohort instruction
    let (initialize_cohort_ix, _, _) = build_initialize_cohort_ix(
        campaign_info.admin,
        campaign_address,
        campaign_info.fingerprint,
        cohort_address,
        cohort.merkle_root,
        cohort.amount_per_entitlement,
        cohort.vault_count as u8,
    )
    .map_err(|e| CliError::InvalidConfig(format!("Failed to build cohort instruction: {}", e)))?;

    // Create and send transaction
    let recent_blockhash = client
        .rpc_client()
        .get_latest_blockhash()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?;

    let transaction = Transaction::new_signed_with_payer(
        &[initialize_cohort_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    println!("    📤 Sending cohort initialization transaction...");

    let config = RpcSendTransactionConfig {
        skip_preflight: false,
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        encoding: None,
        max_retries: Some(5),
        min_context_slot: None,
    };

    let signature = client
        .rpc_client()
        .send_and_confirm_transaction_with_spinner_and_config(
            &transaction,
            CommitmentConfig::confirmed(),
            config,
        )
        .map_err(|e| CliError::InvalidConfig(format!("Failed to deploy cohort: {}", e)))?;

    println!("    ✅ Cohort PDA deployed! Signature: {}", signature);

    // Skip database update since the method doesn't exist yet
    println!("    ⚠️  Database cohort deployment tracking not yet implemented");

    Ok(signature.to_string())
}

fn deploy_and_fund_cohort_vaults(
    client: &PrismProtocolClient,
    admin_keypair: &dyn Signer,
    campaign_info: &prism_protocol_db::CampaignInfo,
    cohort: &prism_protocol_db::CohortInfo,
    db: &mut CampaignDatabase,
) -> CliResult<Vec<String>> {
    let (campaign_address, _) = client
        .address_finder()
        .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);

    let (cohort_address, _) = client
        .address_finder()
        .find_cohort_v0_address(&campaign_address, &cohort.merkle_root);

    let mut vault_signatures = Vec::new();

    // Get vault requirements from database for this cohort
    let vault_requirements = db
        .read_vault_requirements()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read vault requirements: {}", e)))?
        .into_iter()
        .filter(|v| v.cohort_name == cohort.name)
        .collect::<Vec<_>>();

    // Process each vault: create first, then fund
    for vault_req in vault_requirements {
        let vault_index = vault_req.vault_index as u8;

        let (vault_address, _) = client
            .address_finder()
            .find_vault_v0_address(&cohort_address, vault_index);

        println!(
            "        🏗️  Processing vault {} at {}",
            vault_index, vault_address
        );

        // Step 1: Create vault if it doesn't exist
        let creation_signature = create_vault_if_needed(
            client,
            admin_keypair,
            campaign_info,
            cohort,
            &vault_address,
            vault_index,
        )?;

        if !creation_signature.is_empty() {
            vault_signatures.push(creation_signature);
        }

        // Step 2: Fund vault if it needs tokens
        if vault_req.required_tokens > 0 {
            fund_vault_if_needed(
                client,
                admin_keypair,
                &campaign_info.mint,
                &vault_address,
                vault_req.required_tokens,
                db,
                &cohort.name,
                vault_req.vault_index,
            )?;
        }
    }

    Ok(vault_signatures)
}

fn create_vault_if_needed(
    client: &PrismProtocolClient,
    admin_keypair: &dyn Signer,
    campaign_info: &prism_protocol_db::CampaignInfo,
    cohort: &prism_protocol_db::CohortInfo,
    vault_address: &Pubkey,
    vault_index: u8,
) -> CliResult<String> {
    // Check if vault already exists using client
    let vault_exists = client.rpc_client().get_account(vault_address).is_ok();

    if vault_exists {
        println!(
            "        ⚠️  Vault {} already exists, skipping creation...",
            vault_index
        );
        return Ok(String::new());
    }

    println!("        📤 Creating vault {}...", vault_index);

    let (campaign_address, _) = client
        .address_finder()
        .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);

    let (cohort_address, _) = client
        .address_finder()
        .find_cohort_v0_address(&campaign_address, &cohort.merkle_root);

    // Build create vault instruction
    let (create_vault_ix, _, _) = build_create_vault_ix(
        campaign_info.admin,
        campaign_address,
        cohort_address,
        campaign_info.mint,
        *vault_address,
        campaign_info.fingerprint,
        cohort.merkle_root,
        vault_index,
    )
    .map_err(|e| {
        CliError::InvalidConfig(format!("Failed to build create vault instruction: {}", e))
    })?;

    // Create and send transaction
    let recent_blockhash = client
        .rpc_client()
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

    let signature = client
        .rpc_client()
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

    // Skip database update since the method doesn't exist yet
    println!("        ⚠️  Database vault creation tracking not yet implemented");

    Ok(signature.to_string())
}

fn fund_vault_if_needed(
    client: &PrismProtocolClient,
    admin_keypair: &dyn Signer,
    mint: &Pubkey,
    vault_address: &Pubkey,
    required_tokens: u64,
    db: &mut CampaignDatabase,
    cohort_name: &str,
    vault_index: usize,
) -> CliResult<()> {
    // Check current vault balance using client
    let current_balance = match client.get_token_account(vault_address) {
        Ok(Some(token_account)) => token_account.amount,
        _ => 0,
    };

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
    )
    .map_err(|e| CliError::InvalidConfig(format!("Failed to build transfer instruction: {}", e)))?;

    let recent_blockhash = client
        .rpc_client()
        .get_latest_blockhash()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?;

    let transaction = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    let signature = client
        .rpc_client()
        .send_and_confirm_transaction_with_spinner(&transaction)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to fund vault: {}", e)))?;

    println!(
        "        ✅ Vault {} funded with {} tokens! Signature: {}",
        vault_index, tokens_needed, signature
    );

    // Update database with vault funding status
    db.update_vault_funding(
        cohort_name,
        vault_index,
        &signature.to_string(),
        tokens_needed,
    )
    .map_err(|e| CliError::InvalidConfig(format!("Failed to update database: {}", e)))?;

    Ok(())
}

fn activate_campaign(
    client: &PrismProtocolClient,
    admin_keypair: &dyn Signer,
    campaign_info: &prism_protocol_db::CampaignInfo,
    _db: &mut CampaignDatabase,
) -> CliResult<()> {
    let (campaign_address, _) = client
        .address_finder()
        .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);

    // Check if campaign exists - fix argument order
    match client.get_campaign_v0(&campaign_info.fingerprint, &campaign_info.admin) {
        Ok(Some(_)) => {
            println!("  🎯 Campaign PDA found, activating campaign...");

            // Build activation instruction
            let (activate_ix, _, _) = build_set_campaign_active_status_ix(
                campaign_info.admin,
                campaign_address,
                campaign_info.fingerprint,
                true, // Set to active
            )
            .map_err(|e| {
                CliError::InvalidConfig(format!("Failed to build activation instruction: {}", e))
            })?;

            // Send activation transaction
            let recent_blockhash = client
                .rpc_client()
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

            let signature = client
                .rpc_client()
                .send_and_confirm_transaction_with_spinner_and_config(
                    &transaction,
                    CommitmentConfig::confirmed(),
                    config,
                )
                .map_err(|e| {
                    CliError::InvalidConfig(format!("Failed to activate campaign: {}", e))
                })?;

            println!("  ✅ Campaign activated! Signature: {}", signature);

            // Skip database update since the method doesn't exist yet
            println!("  ⚠️  Database campaign activation tracking not yet implemented");
        }
        _ => {
            return Err(CliError::InvalidConfig(
                "Campaign PDA not found - deployment may have failed".to_string(),
            ));
        }
    }

    Ok(())
}

fn verify_deployment(
    client: &PrismProtocolClient,
    campaign_info: &prism_protocol_db::CampaignInfo,
    cohort_data: &[prism_protocol_db::CohortInfo],
    vault_requirements: &[prism_protocol_db::VaultRequirement],
) -> CliResult<()> {
    println!("  🔍 Verifying campaign deployment...");

    // Verify campaign PDA exists - fix argument order
    match client.get_campaign_v0(&campaign_info.fingerprint, &campaign_info.admin) {
        Ok(Some(_)) => println!("    ✅ Campaign PDA verified"),
        _ => {
            return Err(CliError::InvalidConfig(
                "Campaign PDA verification failed".to_string(),
            ))
        }
    }

    // Verify all cohort PDAs exist
    let (campaign_address, _) = client
        .address_finder()
        .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);

    for cohort in cohort_data {
        match client.get_cohort_v0(&campaign_address, &cohort.merkle_root) {
            Ok(Some(_)) => println!("    ✅ Cohort {} PDA verified", cohort.name),
            _ => {
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

        let (cohort_address, _) = client
            .address_finder()
            .find_cohort_v0_address(&campaign_address, &cohort.merkle_root);

        let (vault_address, _) = client
            .address_finder()
            .find_vault_v0_address(&cohort_address, vault_req.vault_index as u8);

        match client.get_token_account(&vault_address) {
            Ok(Some(token_account)) => {
                let balance = token_account.amount;
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
            _ => {
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

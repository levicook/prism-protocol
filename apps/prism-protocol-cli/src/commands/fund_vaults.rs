use rusqlite::Connection;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token;
use std::path::PathBuf;
use std::str::FromStr;

use crate::error::{CliError, CliResult};
use prism_protocol_sdk::{
    address_finders::{find_campaign_address, find_cohort_v0_address, find_vault_v0_address},
    instruction_builders::build_create_vault_ix,
};

#[derive(Debug)]
struct VaultFundingData {
    cohort_name: String,
    vault_index: usize,
    required_tokens: u64,
    assigned_claimants: usize,
    merkle_root: [u8; 32],
}

pub fn execute(campaign_db_in: PathBuf, admin_keypair: PathBuf, rpc_url: String) -> CliResult<()> {
    println!("🏦 Creating and funding campaign vaults...");
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
        .map_err(|e| CliError::Rpc(format!("Failed to connect to RPC: {}", e)))?;
    println!("✅ Connected to Solana RPC: {}", rpc_url);

    // Step 3: Read campaign data from database
    println!("\n📋 Reading campaign data from database...");
    let conn = Connection::open(&campaign_db_in)?;

    let campaign_data = read_campaign_data(&conn)?;
    println!(
        "✅ Campaign fingerprint: {}",
        hex::encode(campaign_data.fingerprint)
    );
    println!("✅ Mint: {}", campaign_data.mint);

    // Step 4: Read vault funding requirements
    println!("\n💰 Reading vault funding requirements...");
    let vault_requirements = read_vault_requirements(&conn)?;
    let total_tokens_needed: u64 = vault_requirements.iter().map(|v| v.required_tokens).sum();
    println!(
        "✅ Found {} vaults requiring {} total tokens",
        vault_requirements.len(),
        total_tokens_needed
    );

    // Step 5: Create and fund vaults
    println!("\n🏗️  Creating and funding vaults...");
    println!(
        "📊 Progress: 0/{} vaults processed",
        vault_requirements.len()
    );

    for (index, vault_req) in vault_requirements.iter().enumerate() {
        create_and_fund_vault(
            &rpc_client,
            &admin_keypair,
            &campaign_data,
            vault_req,
            &campaign_db_in,
        )?;

        println!(
            "📊 Progress: {}/{} vaults processed",
            index + 1,
            vault_requirements.len()
        );
    }

    println!("\n🎉 Vault creation and funding completed successfully!");
    println!("📊 Summary:");
    println!("  - {} vaults created and funded", vault_requirements.len());
    println!("  - {} total tokens distributed", total_tokens_needed);
    println!("  - Database updated with vault PDA addresses and funding status");

    Ok(())
}

#[derive(Debug)]
struct CampaignData {
    fingerprint: [u8; 32],
    mint: Pubkey,
    admin: Pubkey,
}

fn read_campaign_data(conn: &Connection) -> CliResult<CampaignData> {
    let mut stmt = conn.prepare("SELECT fingerprint, mint, admin FROM campaign")?;
    let mut rows = stmt.query_map([], |row| {
        let fingerprint_hex: String = row.get(0)?;
        let mint_str: String = row.get(1)?;
        let admin_str: String = row.get(2)?;

        Ok((fingerprint_hex, mint_str, admin_str))
    })?;

    if let Some(row) = rows.next() {
        let (fingerprint_hex, mint_str, admin_str) = row?;

        let fingerprint = hex::decode(&fingerprint_hex)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid fingerprint hex: {}", e)))?
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
            "No campaign found in database".to_string(),
        ))
    }
}

fn read_vault_requirements(conn: &Connection) -> CliResult<Vec<VaultFundingData>> {
    let mut stmt = conn.prepare(
        "SELECT v.cohort_name, v.vault_index, v.required_tokens, v.assigned_claimants, c.merkle_root
         FROM vaults v
         JOIN cohorts c ON v.cohort_name = c.cohort_name
         WHERE v.vault_pubkey IS NULL 
         ORDER BY v.cohort_name, v.vault_index",
    )?;

    let vault_iter = stmt.query_map([], |row| {
        let merkle_root_hex: String = row.get(4)?;
        let merkle_root = hex::decode(&merkle_root_hex)
            .map_err(|_e| {
                rusqlite::Error::InvalidColumnType(
                    4,
                    "merkle_root".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .try_into()
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    4,
                    "merkle_root".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;

        Ok(VaultFundingData {
            cohort_name: row.get(0)?,
            vault_index: row.get(1)?,
            required_tokens: row.get(2)?,
            assigned_claimants: row.get(3)?,
            merkle_root,
        })
    })?;

    let mut vaults = Vec::new();
    for vault in vault_iter {
        vaults.push(vault?);
    }

    Ok(vaults)
}

fn create_and_fund_vault(
    rpc_client: &RpcClient,
    admin_keypair: &Keypair,
    campaign_data: &CampaignData,
    vault_req: &VaultFundingData,
    campaign_db_path: &PathBuf,
) -> CliResult<()> {
    println!(
        "  🏦 Creating vault for cohort: {} (index: {})",
        vault_req.cohort_name, vault_req.vault_index
    );
    println!("    💰 Required tokens: {}", vault_req.required_tokens);
    println!(
        "    👥 Assigned claimants: {}",
        vault_req.assigned_claimants
    );

    // Step 1: Derive campaign, cohort, and vault PDA addresses
    let (campaign_address, _) =
        find_campaign_address(&campaign_data.admin, &campaign_data.fingerprint);
    let (cohort_address, _) = find_cohort_v0_address(&campaign_address, &vault_req.merkle_root);
    let (vault_pda, _) = find_vault_v0_address(&cohort_address, vault_req.vault_index as u8);

    println!("    📍 Campaign PDA: {}", campaign_address);
    println!("    📍 Cohort PDA: {}", cohort_address);
    println!("    📍 Vault PDA: {}", vault_pda);

    // Step 2: Build create vault instruction (this creates the token account owned by the vault PDA)
    let (create_vault_ix, _, _) = build_create_vault_ix(
        admin_keypair.pubkey(),
        campaign_address,
        cohort_address,
        campaign_data.mint,
        vault_pda,
        campaign_data.fingerprint,
        vault_req.merkle_root,
        vault_req.vault_index as u8,
    )
    .map_err(|e| {
        CliError::InvalidConfig(format!("Failed to build create vault instruction: {}", e))
    })?;

    // Step 3: Send create vault transaction
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
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
        max_retries: None,
        min_context_slot: None,
    };

    let signature = rpc_client.send_and_confirm_transaction_with_spinner_and_config(
        &transaction,
        CommitmentConfig::confirmed(),
        config,
    )?;

    println!("    ✅ Vault created! Signature: {}", signature);

    // Step 4: Fund the vault's token account if needed
    if vault_req.required_tokens > 0 {
        fund_vault(
            rpc_client,
            admin_keypair,
            &campaign_data.mint,
            &vault_pda,
            vault_req.required_tokens,
        )?;
    }

    // Step 5: Update database with vault PDA address
    update_vault_in_database(
        campaign_db_path,
        &vault_req.cohort_name,
        vault_req.vault_index,
        &vault_pda,
        &signature.to_string(),
    )?;

    Ok(())
}

fn fund_vault(
    rpc_client: &RpcClient,
    admin_keypair: &Keypair,
    mint: &Pubkey,
    vault_pda: &Pubkey,
    amount: u64,
) -> CliResult<()> {
    println!(
        "    💰 Funding vault's token account with {} tokens...",
        amount
    );

    // For wrapped SOL, we need to transfer SOL and then sync native
    if mint == &spl_token::native_mint::ID {
        fund_vault_with_wrapped_sol(rpc_client, admin_keypair, vault_pda, amount)
    } else {
        // For other tokens, we need to mint them (assuming admin is mint authority)
        fund_vault_with_spl_tokens(rpc_client, admin_keypair, mint, vault_pda, amount)
    }
}

fn fund_vault_with_wrapped_sol(
    rpc_client: &RpcClient,
    admin_keypair: &Keypair,
    vault: &Pubkey,
    amount: u64,
) -> CliResult<()> {
    // Transfer SOL to the vault
    let transfer_ix = system_instruction::transfer(&admin_keypair.pubkey(), vault, amount);

    // Sync native to update the vault balance
    let sync_native_ix = spl_token::instruction::sync_native(&spl_token::ID, vault)?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_ix, sync_native_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    let signature = rpc_client.send_and_confirm_transaction_with_spinner(&transaction)?;
    println!(
        "    ✅ Vault funded with wrapped SOL! Signature: {}",
        signature
    );

    Ok(())
}

fn fund_vault_with_spl_tokens(
    rpc_client: &RpcClient,
    admin_keypair: &Keypair,
    mint: &Pubkey,
    vault: &Pubkey,
    amount: u64,
) -> CliResult<()> {
    // Mint tokens directly to the vault (assuming admin is mint authority)
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::ID,
        mint,
        vault,
        &admin_keypair.pubkey(), // Assuming admin is mint authority
        &[],
        amount,
    )?;

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&admin_keypair.pubkey()),
        &[admin_keypair],
        recent_blockhash,
    );

    let signature = rpc_client.send_and_confirm_transaction_with_spinner(&transaction)?;
    println!(
        "    ✅ Vault funded with SPL tokens! Signature: {}",
        signature
    );

    Ok(())
}

fn update_vault_in_database(
    campaign_db_path: &PathBuf,
    cohort_name: &str,
    vault_index: usize,
    vault_pda: &Pubkey,
    creation_signature: &str,
) -> CliResult<()> {
    let conn = Connection::open(campaign_db_path)?;

    conn.execute(
        "UPDATE vaults SET vault_pubkey = ?, created_at = ?, creation_signature = ? 
         WHERE cohort_name = ? AND vault_index = ?",
        (
            vault_pda.to_string(),
            chrono::Utc::now().timestamp(),
            creation_signature,
            cohort_name,
            vault_index,
        ),
    )?;

    Ok(())
}

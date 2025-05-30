use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_sdk::AddressFinder;
use rusqlite::Connection;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
struct CampaignInfo {
    fingerprint: [u8; 32],
    campaign_address: Pubkey,
    admin: Pubkey,
    mint: Pubkey,
    exists: bool,
    account_data_length: Option<usize>,
}

#[derive(Debug)]
struct CohortStatus {
    name: String,
    merkle_root: [u8; 32],
    cohort_address: Pubkey,
    exists: bool,
    account_data_length: Option<usize>,
    vaults: Vec<VaultStatus>,
}

#[derive(Debug)]
struct VaultStatus {
    index: u8,
    vault_address: Pubkey,
    exists: bool,
    token_balance: u64,
    #[allow(dead_code)]
    account_data_length: Option<usize>,
}

pub fn execute(campaign_db_in: PathBuf, rpc_url: String) -> CliResult<()> {
    println!("🔍 Querying campaign status on-chain...");
    println!("📊 Database: {}", campaign_db_in.display());

    // Setup RPC client
    let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());

    // Read campaign info from database
    let campaign_info = read_campaign_info(&campaign_db_in, &rpc_client)?;
    print_campaign_info(&campaign_info);

    // Query cohorts if campaign exists
    if campaign_info.exists {
        let cohort_statuses = query_cohort_statuses(&rpc_client, &campaign_db_in, &campaign_info)?;
        print_cohort_statuses(&cohort_statuses);
    } else {
        println!("\n⚠️  Campaign not deployed - skipping cohort/vault checks");
        println!("   Run `deploy-campaign` to create on-chain accounts");
    }

    Ok(())
}

fn read_campaign_info(db_path: &PathBuf, rpc_client: &RpcClient) -> CliResult<CampaignInfo> {
    let address_finder = AddressFinder::default();
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

        // Derive campaign address and check if it exists
        let (campaign_address, _) = address_finder.find_campaign_v0_address(&admin, &fingerprint);

        let (exists, account_data_length) = match rpc_client.get_account(&campaign_address) {
            Ok(account) => (true, Some(account.data.len())),
            Err(_) => (false, None),
        };

        Ok(CampaignInfo {
            fingerprint,
            campaign_address,
            admin,
            mint,
            exists,
            account_data_length,
        })
    } else {
        Err(CliError::InvalidConfig(
            "No campaign data found in database".to_string(),
        ))
    }
}

fn query_cohort_statuses(
    rpc_client: &RpcClient,
    db_path: &PathBuf,
    campaign_info: &CampaignInfo,
) -> CliResult<Vec<CohortStatus>> {
    let conn = Connection::open(db_path)?;
    let mut cohort_statuses = Vec::new();

    // Get cohort info from database
    let mut stmt = conn.prepare(
        "SELECT DISTINCT c.cohort_name, h.merkle_root 
         FROM claimants c
         JOIN cohorts h ON c.cohort_name = h.cohort_name 
         ORDER BY c.cohort_name",
    )?;

    let rows = stmt.query_map([], |row| {
        let cohort_name: String = row.get(0)?;
        let merkle_root_hex: String = row.get(1)?;
        Ok((cohort_name, merkle_root_hex))
    })?;

    for row in rows {
        if let Ok((cohort_name, merkle_root_hex)) = row {
            if let Ok(merkle_root_bytes) = hex::decode(&merkle_root_hex) {
                if let Ok(merkle_root) = merkle_root_bytes.try_into() as Result<[u8; 32], _> {
                    let cohort_status = query_single_cohort_status(
                        rpc_client,
                        &campaign_info.campaign_address,
                        &cohort_name,
                        &merkle_root,
                        db_path,
                    )?;
                    cohort_statuses.push(cohort_status);
                }
            }
        }
    }

    Ok(cohort_statuses)
}

fn query_single_cohort_status(
    rpc_client: &RpcClient,
    campaign_address: &Pubkey,
    cohort_name: &str,
    merkle_root: &[u8; 32],
    db_path: &PathBuf,
) -> CliResult<CohortStatus> {
    let address_finder = AddressFinder::default();
    let (cohort_address, _) = address_finder.find_cohort_v0_address(campaign_address, merkle_root);

    let (exists, account_data_length) = match rpc_client.get_account(&cohort_address) {
        Ok(account) => (true, Some(account.data.len())),
        Err(_) => (false, None),
    };

    // Query vault statuses from database first
    let mut vaults = Vec::new();

    // Get actual vault data from database
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT vault_index, vault_pubkey FROM vaults WHERE cohort_name = ? ORDER BY vault_index",
    )?;

    let vault_rows = stmt.query_map([cohort_name], |row| {
        let vault_index: u8 = row.get::<_, i64>(0)? as u8;
        let vault_pubkey_str: String = row.get(1)?;
        Ok((vault_index, vault_pubkey_str))
    })?;

    for vault_row in vault_rows {
        if let Ok((vault_index, vault_pubkey_str)) = vault_row {
            if let Ok(vault_address) = Pubkey::from_str(&vault_pubkey_str) {
                match rpc_client.get_account(&vault_address) {
                    Ok(account) => {
                        let token_balance = if account.data.len() >= 72 {
                            // Token account amount is at bytes 64-72 (u64 little endian)
                            let amount_bytes: [u8; 8] =
                                account.data[64..72].try_into().unwrap_or([0u8; 8]);
                            u64::from_le_bytes(amount_bytes)
                        } else {
                            0
                        };

                        vaults.push(VaultStatus {
                            index: vault_index,
                            vault_address,
                            exists: true,
                            token_balance,
                            account_data_length: Some(account.data.len()),
                        });
                    }
                    Err(_) => {
                        vaults.push(VaultStatus {
                            index: vault_index,
                            vault_address,
                            exists: false,
                            token_balance: 0,
                            account_data_length: None,
                        });
                    }
                }
            }
        }
    }

    Ok(CohortStatus {
        name: cohort_name.to_string(),
        merkle_root: *merkle_root,
        cohort_address,
        exists,
        account_data_length,
        vaults,
    })
}

fn print_campaign_info(info: &CampaignInfo) {
    println!("\n🏛️  Campaign Information:");
    println!("   Fingerprint: {}", hex::encode(info.fingerprint));
    println!("   Admin: {}", info.admin);
    println!("   Mint: {}", info.mint);
    println!("   Address: {}", info.campaign_address);

    if info.exists {
        println!("   ✅ Status: EXISTS");
        if let Some(len) = info.account_data_length {
            println!("   📏 Data length: {} bytes", len);
        }
    } else {
        println!("   ❌ Status: NOT FOUND");
        println!("   💡 Campaign may not be deployed yet");
    }
}

fn print_cohort_statuses(cohorts: &[CohortStatus]) {
    if cohorts.is_empty() {
        println!("\n📂 No cohorts found in campaign database");
        return;
    }

    println!("\n📂 Cohorts ({}):", cohorts.len());

    for (i, cohort) in cohorts.iter().enumerate() {
        println!("\n   {}. Cohort: {}", i + 1, cohort.name);
        println!("      Address: {}", cohort.cohort_address);
        println!("      Merkle Root: {}", hex::encode(cohort.merkle_root));

        if cohort.exists {
            println!("      ✅ Status: EXISTS");
            if let Some(len) = cohort.account_data_length {
                println!("      📏 Data length: {} bytes", len);
            }
        } else {
            println!("      ❌ Status: NOT FOUND");
        }

        // Print vault statuses
        if !cohort.vaults.is_empty() {
            println!("      💰 Vaults:");
            for vault in &cohort.vaults {
                if vault.exists {
                    println!(
                        "         Vault {}: ✅ EXISTS - {} tokens ({})",
                        vault.index, vault.token_balance, vault.vault_address
                    );
                } else {
                    println!(
                        "         Vault {}: ❌ NOT FOUND ({})",
                        vault.index, vault.vault_address
                    );
                }
            }
        }
    }
}

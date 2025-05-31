use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_client::PrismProtocolClient;
use prism_protocol_db::CampaignDatabase;
use solana_sdk::pubkey::Pubkey;
use std::path::PathBuf;

#[derive(Debug)]
struct CampaignStatusReport {
    campaign: CampaignInfo,
    cohorts: Vec<CohortStatus>,
}

#[derive(Debug)]
struct CampaignInfo {
    fingerprint: [u8; 32],
    campaign_address: Pubkey,
    admin: Pubkey,
    mint: Pubkey,
    exists: bool,
    #[allow(dead_code)]
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

pub fn execute(campaign_db_path: PathBuf, rpc_url: String) -> CliResult<()> {
    println!("🔍 Querying campaign status on-chain...");
    println!("📊 Database: {}", campaign_db_path.display());

    // Open database and create unified client
    let db = CampaignDatabase::open(&campaign_db_path)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to open database: {}", e)))?;
    let client = PrismProtocolClient::new(rpc_url)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to create RPC client: {}", e)))?;

    // Generate status report
    let report = generate_status_report(&db, &client)?;

    // Print results
    print_status_report(&report);

    Ok(())
}

fn generate_status_report(
    db: &CampaignDatabase,
    client: &PrismProtocolClient,
) -> CliResult<CampaignStatusReport> {
    // Get campaign info from database
    let campaign_data = db
        .read_campaign_info()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read campaign: {}", e)))?;

    let (campaign_address, _) = client
        .address_finder()
        .find_campaign_v0_address(&campaign_data.admin, &campaign_data.fingerprint);

    // Check if campaign exists on-chain using the client's typed method
    let campaign_account = client
        .get_campaign_v0(&campaign_data.fingerprint, &campaign_data.admin)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to query campaign: {}", e)))?;

    let exists = campaign_account.is_some();
    let account_data_length = if exists { Some(0) } else { None }; // We don't need raw data length anymore

    let campaign_info = CampaignInfo {
        fingerprint: campaign_data.fingerprint,
        campaign_address,
        admin: campaign_data.admin,
        mint: campaign_data.mint,
        exists,
        account_data_length,
    };

    // Get cohort statuses
    let cohorts = if exists {
        generate_cohort_statuses(db, client, &campaign_address)?
    } else {
        Vec::new()
    };

    Ok(CampaignStatusReport {
        campaign: campaign_info,
        cohorts,
    })
}

fn generate_cohort_statuses(
    db: &CampaignDatabase,
    client: &PrismProtocolClient,
    campaign_address: &Pubkey,
) -> CliResult<Vec<CohortStatus>> {
    let mut cohorts = Vec::new();

    // Get cohort info from database
    let cohort_data = db
        .read_cohorts()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read cohorts: {}", e)))?;

    for cohort in cohort_data {
        let (cohort_address, _) = client
            .address_finder()
            .find_cohort_v0_address(campaign_address, &cohort.merkle_root);

        // Check if cohort exists on-chain using the client's typed method
        let cohort_account = client
            .get_cohort_v0(campaign_address, &cohort.merkle_root)
            .map_err(|e| CliError::InvalidConfig(format!("Failed to query cohort: {}", e)))?;

        let exists = cohort_account.is_some();
        let account_data_length = if exists { Some(0) } else { None }; // We don't need raw data length anymore

        // Get vault statuses for this cohort using the client's typed methods
        let vaults =
            generate_vault_statuses_from_addresses(client, &cohort_address, &cohort.vaults)?;

        cohorts.push(CohortStatus {
            name: cohort.name,
            merkle_root: cohort.merkle_root,
            cohort_address,
            exists,
            account_data_length,
            vaults,
        });
    }

    Ok(cohorts)
}

fn generate_vault_statuses_from_addresses(
    client: &PrismProtocolClient,
    _cohort_address: &Pubkey,
    vault_addresses: &[Pubkey],
) -> CliResult<Vec<VaultStatus>> {
    let mut vaults = Vec::new();

    for (index, &vault_address) in vault_addresses.iter().enumerate() {
        // Use the client's typed token account method
        let token_account = client
            .get_token_account(&vault_address)
            .map_err(|e| CliError::InvalidConfig(format!("Failed to query vault: {}", e)))?;

        if let Some(account) = token_account {
            vaults.push(VaultStatus {
                index: index as u8,
                vault_address,
                exists: true,
                token_balance: account.amount,
                account_data_length: Some(0), // We don't need raw data length anymore
            });
        } else {
            vaults.push(VaultStatus {
                index: index as u8,
                vault_address,
                exists: false,
                token_balance: 0,
                account_data_length: None,
            });
        }
    }

    Ok(vaults)
}

fn print_status_report(report: &CampaignStatusReport) {
    print_campaign_info(&report.campaign);

    if report.campaign.exists {
        print_cohort_statuses(&report.cohorts);
    } else {
        println!("\n⚠️  Campaign not deployed - skipping cohort/vault checks");
        println!("   Run `deploy-campaign` to create on-chain accounts");
    }
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

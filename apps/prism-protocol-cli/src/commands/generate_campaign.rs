use crate::error::{CliError, CliResult};
use csv::Reader;
use rusqlite::Connection;
use serde::Deserialize;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
struct CampaignRow {
    cohort: String,
    claimant: String,
    entitlements: u64,
}

#[derive(Debug, Deserialize)]
struct CohortConfigRow {
    cohort: String,
    amount_per_entitlement: u64,
}

#[derive(Debug, Clone)]
struct ClaimantData {
    claimant: Pubkey,
    entitlements: u64,
}

#[derive(Debug)]
struct CohortData {
    name: String,
    amount_per_entitlement: u64,
    claimants: Vec<ClaimantData>,
    vault_count: usize,
}

pub fn execute(
    campaign_csv_in: PathBuf,
    cohorts_csv_in: PathBuf,
    mint: Pubkey,
    admin_keypair: PathBuf,
    claimants_per_vault: usize,
    campaign_db_out: PathBuf,
) -> CliResult<()> {
    println!("🚀 Generating campaign...");
    println!("Campaign file: {}", campaign_csv_in.display());
    println!("Cohorts file: {}", cohorts_csv_in.display());
    println!("Mint: {}", mint);
    println!("Admin keypair: {}", admin_keypair.display());
    println!("Claimants per vault: {}", claimants_per_vault);
    println!("Output database: {}", campaign_db_out.display());

    // Step 0: Read and validate admin keypair
    println!("\n🔑 Reading admin keypair...");
    let admin_keypair = read_keypair_file(&admin_keypair)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read admin keypair: {}", e)))?;
    let admin_pubkey = admin_keypair.pubkey();
    println!("✅ Admin public key: {}", admin_pubkey);

    // Step 1: Parse campaign CSV file
    println!("\n📋 Parsing campaign claimants...");
    let campaign_rows = parse_campaign_csv(&campaign_csv_in)?;
    println!("✅ Loaded {} claimant entries", campaign_rows.len());

    // Step 2: Parse cohorts CSV file
    println!("\n📋 Parsing cohort configuration...");
    let cohort_configs = parse_cohorts_csv(&cohorts_csv_in)?;
    println!("✅ Loaded {} cohort configurations", cohort_configs.len());

    // Step 3: Validate cohort consistency
    println!("\n🔍 Validating cohort consistency...");
    validate_cohort_consistency(&campaign_rows, &cohort_configs)?;
    println!("✅ All cohorts are consistent between files");

    // Step 4: Group claimants by cohort and calculate vault counts
    println!("\n📊 Processing cohorts...");
    let cohort_data = process_cohorts(&campaign_rows, &cohort_configs, claimants_per_vault)?;

    for cohort in &cohort_data {
        println!(
            "  📦 {}: {} claimants, {} vaults",
            cohort.name,
            cohort.claimants.len(),
            cohort.vault_count
        );
    }

    // Step 5: Create SQLite database
    println!("\n💾 Creating campaign database...");
    create_campaign_database(&campaign_db_out, &mint, &admin_pubkey, &cohort_data)?;
    println!(
        "✅ Campaign database created: {}",
        campaign_db_out.display()
    );

    // TODO: Next steps
    // 6. Generate merkle trees for each cohort
    // 7. Calculate campaign fingerprint
    // 8. Populate database with merkle data
    // 9. Verify admin keypair has sufficient token balance

    println!("\n🎉 Campaign generation completed!");

    Ok(())
}

fn parse_campaign_csv(path: &PathBuf) -> CliResult<Vec<CampaignRow>> {
    let file = File::open(path)?;
    let mut reader = Reader::from_reader(file);
    let mut rows = Vec::new();

    for result in reader.deserialize() {
        let row: CampaignRow = result?;
        rows.push(row);
    }

    Ok(rows)
}

fn parse_cohorts_csv(path: &PathBuf) -> CliResult<Vec<CohortConfigRow>> {
    let file = File::open(path)?;
    let mut reader = Reader::from_reader(file);
    let mut rows = Vec::new();

    for result in reader.deserialize() {
        let row: CohortConfigRow = result?;
        rows.push(row);
    }

    Ok(rows)
}

fn validate_cohort_consistency(
    campaign_rows: &[CampaignRow],
    cohort_configs: &[CohortConfigRow],
) -> CliResult<()> {
    // Get unique cohorts from both files
    let campaign_cohorts: HashSet<&String> = campaign_rows.iter().map(|r| &r.cohort).collect();
    let config_cohorts: HashSet<&String> = cohort_configs.iter().map(|r| &r.cohort).collect();

    // Check for missing cohorts in config
    let missing_in_config: Vec<&String> = campaign_cohorts
        .difference(&config_cohorts)
        .cloned()
        .collect();
    if !missing_in_config.is_empty() {
        return Err(CliError::InvalidConfig(format!(
            "Cohorts found in campaign file but missing from cohorts config: {:?}",
            missing_in_config
        )));
    }

    // Check for extra cohorts in config
    let extra_in_config: Vec<&String> = config_cohorts
        .difference(&campaign_cohorts)
        .cloned()
        .collect();
    if !extra_in_config.is_empty() {
        return Err(CliError::InvalidConfig(format!(
            "Cohorts found in cohorts config but missing from campaign file: {:?}",
            extra_in_config
        )));
    }

    Ok(())
}

fn process_cohorts(
    campaign_rows: &[CampaignRow],
    cohort_configs: &[CohortConfigRow],
    claimants_per_vault: usize,
) -> CliResult<Vec<CohortData>> {
    // Create config lookup
    let config_map: HashMap<&String, &CohortConfigRow> = cohort_configs
        .iter()
        .map(|config| (&config.cohort, config))
        .collect();

    // Group claimants by cohort
    let mut cohort_groups: HashMap<String, Vec<ClaimantData>> = HashMap::new();

    for row in campaign_rows {
        let claimant = Pubkey::from_str(&row.claimant)
            .map_err(|_| CliError::InvalidConfig(format!("Invalid pubkey: {}", row.claimant)))?;

        let claimant_data = ClaimantData {
            claimant,
            entitlements: row.entitlements,
        };

        cohort_groups
            .entry(row.cohort.clone())
            .or_default()
            .push(claimant_data);
    }

    // Create cohort data with vault counts
    let mut cohort_data = Vec::new();

    for (cohort_name, claimants) in cohort_groups {
        let config = config_map.get(&cohort_name).unwrap(); // Safe due to validation
        let vault_count = calculate_vault_count(claimants.len(), claimants_per_vault);

        cohort_data.push(CohortData {
            name: cohort_name,
            amount_per_entitlement: config.amount_per_entitlement,
            claimants,
            vault_count,
        });
    }

    // Sort by name for deterministic output
    cohort_data.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(cohort_data)
}

fn calculate_vault_count(claimant_count: usize, claimants_per_vault: usize) -> usize {
    std::cmp::max(
        1,
        (claimant_count + claimants_per_vault - 1) / claimants_per_vault,
    )
}

fn create_campaign_database(
    path: &PathBuf,
    mint: &Pubkey,
    admin_pubkey: &Pubkey,
    cohort_data: &[CohortData],
) -> CliResult<()> {
    let conn = Connection::open(path)?;

    // Create schema
    conn.execute_batch(
        r#"
        CREATE TABLE campaign (
            fingerprint TEXT PRIMARY KEY,
            mint TEXT NOT NULL,
            admin TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            deployed_at INTEGER
        );
        
        CREATE TABLE cohorts (
            cohort_name TEXT PRIMARY KEY,
            merkle_root TEXT,
            amount_per_entitlement INTEGER NOT NULL,
            vault_count INTEGER NOT NULL,
            claimant_count INTEGER NOT NULL,
            total_tokens_required INTEGER NOT NULL,
            deployed_at INTEGER
        );
        
        CREATE TABLE claimants (
            claimant TEXT NOT NULL,
            cohort_name TEXT NOT NULL,
            entitlements INTEGER NOT NULL,
            assigned_vault_index INTEGER,
            merkle_proof TEXT,
            claimed_at INTEGER,
            PRIMARY KEY (claimant, cohort_name),
            FOREIGN KEY (cohort_name) REFERENCES cohorts(cohort_name)
        );
        
        CREATE TABLE vaults (
            cohort_name TEXT NOT NULL,
            vault_index INTEGER NOT NULL,
            vault_pubkey TEXT,
            required_tokens INTEGER NOT NULL,
            assigned_claimants INTEGER NOT NULL,
            funded_at INTEGER,
            PRIMARY KEY (cohort_name, vault_index),
            FOREIGN KEY (cohort_name) REFERENCES cohorts(cohort_name)
        );
    "#,
    )?;

    // Insert initial campaign data (fingerprint will be calculated later)
    conn.execute(
        "INSERT INTO campaign (fingerprint, mint, admin, created_at) VALUES (?, ?, ?, ?)",
        (
            "pending", // Will be updated after merkle tree generation
            mint.to_string(),
            admin_pubkey.to_string(),
            chrono::Utc::now().timestamp(),
        ),
    )?;

    // Insert cohort data
    for cohort in cohort_data {
        let total_tokens_required = cohort
            .claimants
            .iter()
            .map(|c| c.entitlements * cohort.amount_per_entitlement)
            .sum::<u64>();

        conn.execute(
            "INSERT INTO cohorts (cohort_name, amount_per_entitlement, vault_count, claimant_count, total_tokens_required) VALUES (?, ?, ?, ?, ?)",
            (
                &cohort.name,
                cohort.amount_per_entitlement,
                cohort.vault_count,
                cohort.claimants.len(),
                total_tokens_required,
            ),
        )?;

        // Insert claimant data
        for claimant in &cohort.claimants {
            conn.execute(
                "INSERT INTO claimants (claimant, cohort_name, entitlements) VALUES (?, ?, ?)",
                (
                    claimant.claimant.to_string(),
                    &cohort.name,
                    claimant.entitlements,
                ),
            )?;
        }

        // Insert vault placeholders
        for vault_index in 0..cohort.vault_count {
            let claimants_per_vault =
                (cohort.claimants.len() + cohort.vault_count - 1) / cohort.vault_count;
            let start_idx = vault_index * claimants_per_vault;
            let end_idx = std::cmp::min(start_idx + claimants_per_vault, cohort.claimants.len());
            let assigned_claimants = end_idx - start_idx;

            let required_tokens = cohort.claimants[start_idx..end_idx]
                .iter()
                .map(|c| c.entitlements * cohort.amount_per_entitlement)
                .sum::<u64>();

            conn.execute(
                "INSERT INTO vaults (cohort_name, vault_index, required_tokens, assigned_claimants) VALUES (?, ?, ?, ?)",
                (
                    &cohort.name,
                    vault_index,
                    required_tokens,
                    assigned_claimants,
                ),
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_vault_count() {
        assert_eq!(calculate_vault_count(0, 200000), 1);
        assert_eq!(calculate_vault_count(1, 200000), 1);
        assert_eq!(calculate_vault_count(200000, 200000), 1);
        assert_eq!(calculate_vault_count(200001, 200000), 2);
        assert_eq!(calculate_vault_count(400000, 200000), 2);
        assert_eq!(calculate_vault_count(400001, 200000), 3);
    }
}

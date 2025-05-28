use crate::error::{CliError, CliResult};
use csv::Reader;
use hex;
use prism_protocol_merkle::{create_merkle_tree, ClaimMerkleTree};
use prism_protocol_sdk::address_finders::{
    find_campaign_address, find_cohort_v0_address, find_vault_v0_address,
};
use rusqlite::Connection;
use serde::Deserialize;
use sha2::{Digest, Sha256};
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

struct CohortWithMerkle {
    name: String,
    amount_per_entitlement: u64,
    vault_count: usize,
    vaults: Vec<Pubkey>,
    merkle_tree: ClaimMerkleTree,
    merkle_root: [u8; 32],
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

    // Step 5: Generate merkle trees with vault assignments
    println!("\n🌳 Generating merkle trees...");
    let mut cohort_data_with_merkle = Vec::new();

    for cohort in cohort_data {
        println!("  🔄 Processing cohort: {}", cohort.name);

        // Convert claimants to (Pubkey, u64) pairs for merkle tree
        let claimant_pairs: Vec<(Pubkey, u64)> = cohort
            .claimants
            .iter()
            .map(|c| (c.claimant, c.entitlements))
            .collect();

        // Create merkle tree with vault count (no actual vault pubkeys needed yet)
        let merkle_tree = create_merkle_tree(&claimant_pairs, cohort.vault_count)
            .map_err(|e| CliError::InvalidConfig(format!("Failed to create merkle tree: {}", e)))?;

        let merkle_root = merkle_tree
            .root()
            .ok_or_else(|| CliError::InvalidConfig("Failed to get merkle root".to_string()))?;

        // Now derive cohort PDA from campaign and merkle root
        let campaign_address = find_campaign_address(&admin_pubkey, &[0u8; 32]); // Temporary fingerprint
        let (cohort_address, _) = find_cohort_v0_address(&campaign_address.0, &merkle_root);

        // Find vault PDAs for this cohort
        let vaults = find_vault_adresses(&cohort_address, cohort.vault_count);

        println!(
            "    ✅ Generated merkle tree with root: {}",
            hex::encode(merkle_root)
        );

        cohort_data_with_merkle.push(CohortWithMerkle {
            name: cohort.name,
            amount_per_entitlement: cohort.amount_per_entitlement,
            vault_count: cohort.vault_count,
            vaults,
            merkle_tree,
            merkle_root,
        });
    }

    // Step 6: Calculate campaign fingerprint
    println!("\n🔍 Calculating campaign fingerprint...");
    let cohort_roots: Vec<[u8; 32]> = cohort_data_with_merkle
        .iter()
        .map(|c| c.merkle_root)
        .collect();
    let campaign_fingerprint = calculate_campaign_fingerprint(&cohort_roots);
    println!(
        "✅ Campaign fingerprint: {}",
        hex::encode(campaign_fingerprint)
    );

    // Step 7: Create and populate SQLite database
    println!("\n💾 Creating campaign database...");
    create_campaign_database(
        &campaign_db_out,
        &mint,
        &admin_pubkey,
        &cohort_data_with_merkle,
        &campaign_fingerprint,
    )?;
    println!(
        "✅ Campaign database created: {}",
        campaign_db_out.display()
    );

    println!("\n🎉 Campaign generation completed!");
    println!("📊 Summary:");
    println!(
        "  - Campaign fingerprint: {}",
        hex::encode(campaign_fingerprint)
    );
    println!("  - {} cohorts processed", cohort_data_with_merkle.len());
    for cohort in &cohort_data_with_merkle {
        println!(
            "    📦 {}: {} claimants, {} vaults, root: {}",
            cohort.name,
            cohort.merkle_tree.leaves.len(),
            cohort.vault_count,
            hex::encode(cohort.merkle_root)[..8].to_string() + "..."
        );
    }

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

fn find_vault_adresses(cohort_address: &Pubkey, vault_count: usize) -> Vec<Pubkey> {
    let mut vaults = Vec::new();
    for i in 0..vault_count {
        let (vault_address, _) = find_vault_v0_address(cohort_address, i as u8);
        vaults.push(vault_address);
    }
    vaults
}

fn calculate_campaign_fingerprint(cohort_roots: &[[u8; 32]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for root in cohort_roots {
        hasher.update(root);
    }
    hasher.finalize().into()
}

fn create_campaign_database(
    path: &PathBuf,
    mint: &Pubkey,
    admin_pubkey: &Pubkey,
    cohort_data: &[CohortWithMerkle],
    campaign_fingerprint: &[u8; 32],
) -> CliResult<()> {
    let conn = Connection::open(path)?;

    // Create enhanced schema with merkle data
    conn.execute_batch(
        r#"
        CREATE TABLE campaign (
            fingerprint TEXT PRIMARY KEY,
            mint TEXT NOT NULL,
            admin TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            deployed_at INTEGER,
            deployed_signature TEXT -- transaction signature for campaign deployment
        );

        CREATE TABLE cohorts (
            cohort_name TEXT PRIMARY KEY,
            merkle_root TEXT, -- hex-encoded [u8; 32]
            amount_per_entitlement INTEGER NOT NULL,
            vault_count INTEGER NOT NULL,
            claimant_count INTEGER NOT NULL,
            total_tokens_required INTEGER NOT NULL,
            deployed_at INTEGER,
            deployed_signature TEXT -- transaction signature for cohort deployment
        );

        CREATE TABLE claimants (
            claimant TEXT NOT NULL,
            cohort_name TEXT NOT NULL,
            entitlements INTEGER NOT NULL,
            assigned_vault_index INTEGER, -- index into vaults table
            assigned_vault_pubkey TEXT, -- hex-encoded pubkey for convenience
            merkle_proof TEXT, -- hex-encoded proof (comma-separated hashes)
            claimed_at INTEGER,
            PRIMARY KEY (claimant, cohort_name),
            FOREIGN KEY (cohort_name) REFERENCES cohorts(cohort_name)
        );

        CREATE TABLE vaults (
            cohort_name TEXT NOT NULL,
            vault_index INTEGER NOT NULL,
            vault_pubkey TEXT, -- hex-encoded pubkey
            vault_keypair_path TEXT, -- optional: path to keypair file if generated
            required_tokens INTEGER NOT NULL,
            assigned_claimants INTEGER NOT NULL,
            funded_at INTEGER,
            PRIMARY KEY (cohort_name, vault_index),
            FOREIGN KEY (cohort_name) REFERENCES cohorts(cohort_name)
        );

        -- Index for efficient proof lookups
        CREATE INDEX idx_claimants_lookup ON claimants(claimant, cohort_name);
        CREATE INDEX idx_vaults_lookup ON vaults(cohort_name, vault_index);
    "#,
    )?;

    // Insert initial campaign data (fingerprint will be updated after merkle processing)
    conn.execute(
        "INSERT INTO campaign (fingerprint, mint, admin, created_at) VALUES (?, ?, ?, ?)",
        (
            hex::encode(campaign_fingerprint),
            mint.to_string(),
            admin_pubkey.to_string(),
            chrono::Utc::now().timestamp(),
        ),
    )?;

    // Insert cohort data with merkle information
    for cohort in cohort_data {
        let total_tokens_required = cohort
            .merkle_tree
            .leaves
            .iter()
            .map(|leaf| leaf.entitlements * cohort.amount_per_entitlement)
            .sum::<u64>();

        conn.execute(
            "INSERT INTO cohorts (cohort_name, merkle_root, amount_per_entitlement, vault_count, claimant_count, total_tokens_required) VALUES (?, ?, ?, ?, ?, ?)",
            (
                &cohort.name,
                hex::encode(cohort.merkle_root),
                cohort.amount_per_entitlement,
                cohort.vault_count,
                cohort.merkle_tree.leaves.len(),
                total_tokens_required,
            ),
        )?;

        // Insert claimant data with merkle proofs and vault assignments
        for leaf in &cohort.merkle_tree.leaves {
            // Generate merkle proof for this claimant
            let merkle_proof = cohort
                .merkle_tree
                .proof_for_claimant(&leaf.claimant)
                .map_err(|e| CliError::InvalidConfig(format!("Failed to generate proof: {}", e)))?;

            // Get vault pubkey from vault index
            let vault_pubkey = cohort.vaults[leaf.assigned_vault_index as usize];

            // Encode proof as comma-separated hex strings
            let proof_hex = merkle_proof
                .iter()
                .map(|hash| hex::encode(hash))
                .collect::<Vec<_>>()
                .join(",");

            conn.execute(
                "INSERT INTO claimants (claimant, cohort_name, entitlements, assigned_vault_index, assigned_vault_pubkey, merkle_proof) VALUES (?, ?, ?, ?, ?, ?)",
                (
                    leaf.claimant.to_string(),
                    &cohort.name,
                    leaf.entitlements,
                    leaf.assigned_vault_index,
                    vault_pubkey.to_string(),
                    proof_hex,
                ),
            )?;
        }

        // Insert vault data with pubkeys and funding requirements
        for (vault_index, &vault_pubkey) in cohort.vaults.iter().enumerate() {
            // Calculate funding requirements for this vault
            let assigned_claimants = cohort
                .merkle_tree
                .leaves
                .iter()
                .filter(|leaf| leaf.assigned_vault_index as usize == vault_index)
                .count();

            let required_tokens = cohort
                .merkle_tree
                .leaves
                .iter()
                .filter(|leaf| leaf.assigned_vault_index as usize == vault_index)
                .map(|leaf| leaf.entitlements * cohort.amount_per_entitlement)
                .sum::<u64>();

            conn.execute(
                "INSERT INTO vaults (cohort_name, vault_index, vault_pubkey, required_tokens, assigned_claimants) VALUES (?, ?, ?, ?, ?)",
                (
                    &cohort.name,
                    vault_index,
                    vault_pubkey.to_string(),
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

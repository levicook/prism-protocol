/*!
# Campaign Compiler

This module provides core campaign compilation logic, extracting business logic
from CSV inputs and generating the structured data needed for on-chain deployment.

## Key Responsibilities
- Use existing CSV parsing from prism_protocol_csvs
- Generate merkle trees for each cohort
- Calculate campaign fingerprint from merkle roots
- Derive all protocol addresses (campaign, cohorts, vaults)
- Return populated in-memory database ready for use
*/

use crate::AddressFinder;
use prism_protocol_csvs::{
    read_campaign_csv, read_cohorts_csv, validate_csv_consistency, CampaignRow, CohortsRow,
};
use prism_protocol_db::CampaignDatabase;
use prism_protocol_merkle::{create_merkle_tree, ClaimTree};
use sha2::{Digest, Sha256};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::path::Path;

/// Errors that can occur during campaign compilation
#[derive(Debug, thiserror::Error)]
pub enum CompilerError {
    #[error("CSV error: {0}")]
    Csv(#[from] prism_protocol_csvs::CsvError),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Merkle tree generation failed: {0}")]
    MerkleTree(String),
}

pub type CompilerResult<T> = Result<T, CompilerError>;

/// Internal representation of claimant data (claimant, entitlements)
type ClaimantData = (Pubkey, u64);

/// Internal cohort data during processing
#[derive(Debug)]
struct CohortData {
    name: String,
    amount_per_entitlement: u64,
    claimants: Vec<ClaimantData>,
    vault_count: usize,
}

/// Compiled cohort with all derived data
pub struct CompiledCohort {
    pub name: String,
    pub amount_per_entitlement: u64,
    pub vault_count: usize,
    pub vaults: Vec<Pubkey>,
    pub merkle_tree: ClaimTree,
    pub merkle_root: [u8; 32],
    #[allow(dead_code)] // Will be useful for check-campaign CLI
    pub cohort_address: Pubkey,
}

/// Complete compilation result ready for database storage
pub struct CompilationResult {
    pub mint: Pubkey,
    pub admin: Pubkey,
    pub campaign_fingerprint: [u8; 32],
    #[allow(dead_code)] // Will be useful for check-campaign CLI
    pub campaign_address: Pubkey,
    pub cohorts: Vec<CompiledCohort>,
    #[allow(dead_code)] // Will be useful for check-campaign CLI stats
    pub total_claimants: usize,
    #[allow(dead_code)] // Will be useful for check-campaign CLI stats
    pub total_vaults: usize,
}

/// Compile campaign from CSV files into a populated database
///
/// This is the main API for campaign compilation. Takes all parameters and returns
/// a ready-to-use in-memory database. Use db.save_to_file() if you need persistence.
///
/// # Arguments
/// * `address_finder` - For deriving protocol addresses
/// * `campaign_csv` - Path to campaign.csv (cohort, claimant, entitlements)
/// * `cohorts_csv` - Path to cohorts.csv (cohort, amount_per_entitlement)  
/// * `mint` - SPL token mint for the campaign
/// * `admin` - Campaign admin pubkey
/// * `claimants_per_vault` - How many claimants per vault (affects gas costs)
pub fn compile_campaign(
    address_finder: AddressFinder,
    campaign_csv: &Path,
    cohorts_csv: &Path,
    mint: Pubkey,
    admin: Pubkey,
    claimants_per_vault: usize,
) -> CompilerResult<CampaignDatabase> {
    // Step 1: Parse CSV files using existing functions
    let campaign_rows = read_campaign_csv(campaign_csv)?;
    let cohorts_rows = read_cohorts_csv(cohorts_csv)?;

    // Step 2: Validate CSV consistency
    validate_csv_consistency(&campaign_rows, &cohorts_rows)?;

    // Step 3: Process cohorts and calculate vault counts
    let cohort_data = process_cohorts(&campaign_rows, &cohorts_rows, claimants_per_vault)?;

    // Step 4: Generate merkle trees
    let cohort_merkle_data = generate_merkle_trees(cohort_data)?;

    // Step 5: Calculate campaign fingerprint
    let cohort_roots: Vec<[u8; 32]> = cohort_merkle_data
        .iter()
        .map(|(_, _, root)| *root)
        .collect();
    let campaign_fingerprint = calculate_campaign_fingerprint(&cohort_roots);

    // Step 6: Derive addresses using real fingerprint
    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(&admin, &campaign_fingerprint);

    let compiled_cohorts =
        derive_addresses_and_finalize(&address_finder, cohort_merkle_data, &campaign_address)?;

    // Step 7: Calculate totals
    let total_claimants = compiled_cohorts
        .iter()
        .map(|c| c.merkle_tree.leaves.len())
        .sum();
    let total_vaults = compiled_cohorts.iter().map(|c| c.vault_count).sum();

    let compilation_result = CompilationResult {
        mint,
        admin,
        campaign_fingerprint,
        campaign_address,
        cohorts: compiled_cohorts,
        total_claimants,
        total_vaults,
    };

    // Step 8: Create and populate database
    let mut db = CampaignDatabase::create_in_memory()
        .map_err(|e| CompilerError::InvalidConfig(format!("Failed to create database: {}", e)))?;

    populate_database(&mut db, &compilation_result)?;

    Ok(db)
}

/// Process cohorts: group claimants and calculate vault counts
fn process_cohorts(
    campaign_rows: &[CampaignRow],
    cohorts_rows: &[CohortsRow],
    claimants_per_vault: usize,
) -> CompilerResult<Vec<CohortData>> {
    // Create config lookup
    let config_map: HashMap<String, &CohortsRow> = cohorts_rows
        .iter()
        .map(|config| (config.cohort.clone(), config))
        .collect();

    // Group claimants by cohort
    let mut cohort_groups: HashMap<String, Vec<ClaimantData>> = HashMap::new();

    for row in campaign_rows {
        let claimant_data = (row.claimant, row.entitlements);

        cohort_groups
            .entry(row.cohort.clone())
            .or_insert_with(Vec::new)
            .push(claimant_data);
    }

    // Convert to CohortData with vault counts
    let mut cohort_data = Vec::new();
    for (cohort_name, claimants) in cohort_groups {
        let config = config_map.get(&cohort_name).ok_or_else(|| {
            CompilerError::InvalidConfig(format!("No config found for cohort: {}", cohort_name))
        })?;

        // Calculate vault count needed
        let vault_count = (claimants.len() + claimants_per_vault - 1) / claimants_per_vault;

        cohort_data.push(CohortData {
            name: cohort_name,
            amount_per_entitlement: config.amount_per_entitlement,
            claimants,
            vault_count,
        });
    }

    Ok(cohort_data)
}

/// Generate merkle trees for all cohorts
fn generate_merkle_trees(
    cohort_data: Vec<CohortData>,
) -> CompilerResult<Vec<(CohortData, ClaimTree, [u8; 32])>> {
    let mut cohort_merkle_data = Vec::new();

    for cohort in cohort_data {
        // Convert claimants to (Pubkey, u64) pairs for merkle tree
        let claimant_pairs: Vec<(Pubkey, u64)> =
            cohort.claimants.iter().map(|c| c.clone()).collect();

        // Create merkle tree with vault count
        let merkle_tree = create_merkle_tree(&claimant_pairs, cohort.vault_count).map_err(|e| {
            CompilerError::MerkleTree(format!("Failed to create merkle tree: {}", e))
        })?;

        let merkle_root = merkle_tree
            .root()
            .ok_or_else(|| CompilerError::MerkleTree("Failed to get merkle root".to_string()))?;

        cohort_merkle_data.push((cohort, merkle_tree, merkle_root));
    }

    Ok(cohort_merkle_data)
}

/// Calculate campaign fingerprint from merkle roots
fn calculate_campaign_fingerprint(cohort_roots: &[[u8; 32]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for root in cohort_roots {
        hasher.update(root);
    }
    hasher.finalize().into()
}

/// Derive addresses and finalize compilation
fn derive_addresses_and_finalize(
    address_finder: &AddressFinder,
    cohort_merkle_data: Vec<(CohortData, ClaimTree, [u8; 32])>,
    campaign_address: &Pubkey,
) -> CompilerResult<Vec<CompiledCohort>> {
    let mut compiled_cohorts = Vec::new();

    for (cohort, merkle_tree, merkle_root) in cohort_merkle_data {
        // Derive cohort PDA from campaign and merkle root
        let (cohort_address, _) =
            address_finder.find_cohort_v0_address(campaign_address, &merkle_root);

        // Find vault PDAs for this cohort
        let vaults = find_vault_addresses(address_finder, &cohort_address, cohort.vault_count);

        compiled_cohorts.push(CompiledCohort {
            name: cohort.name,
            amount_per_entitlement: cohort.amount_per_entitlement,
            vault_count: cohort.vault_count,
            vaults,
            merkle_tree,
            merkle_root,
            cohort_address,
        });
    }

    Ok(compiled_cohorts)
}

/// Find vault addresses for a cohort
fn find_vault_addresses(
    address_finder: &AddressFinder,
    cohort_address: &Pubkey,
    vault_count: usize,
) -> Vec<Pubkey> {
    (0..vault_count)
        .map(|i| {
            let (vault_address, _) = address_finder.find_vault_v0_address(cohort_address, i as u8);
            vault_address
        })
        .collect()
}

/// Populate database with compilation result
fn populate_database(
    db: &mut CampaignDatabase,
    compilation_result: &CompilationResult,
) -> CompilerResult<()> {
    // Insert campaign info
    db.insert_campaign(
        compilation_result.campaign_fingerprint,
        compilation_result.mint,
        compilation_result.admin,
    )
    .map_err(|e| CompilerError::InvalidConfig(format!("Failed to insert campaign: {}", e)))?;

    // Insert cohorts and related data
    for cohort in &compilation_result.cohorts {
        // Insert cohort
        db.insert_cohort(
            &cohort.name,
            cohort.merkle_root,
            cohort.amount_per_entitlement,
        )
        .map_err(|e| CompilerError::InvalidConfig(format!("Failed to insert cohort: {}", e)))?;

        // Insert claimants for this cohort
        for (index, leaf) in cohort.merkle_tree.leaves.iter().enumerate() {
            let proof = cohort
                .merkle_tree
                .proof_for_claimant(&leaf.claimant)
                .map_err(|e| {
                    CompilerError::MerkleTree(format!(
                        "Failed to generate proof for claimant {}: {}",
                        index, e
                    ))
                })?;

            let proof_hex = proof.iter().map(hex::encode).collect::<Vec<_>>().join(",");

            db.insert_claimant(leaf.claimant, &cohort.name, leaf.entitlements, &proof_hex)
                .map_err(|e| {
                    CompilerError::InvalidConfig(format!("Failed to insert claimant: {}", e))
                })?;
        }

        // Calculate and insert vault requirements
        let total_tokens_for_cohort: u64 = cohort
            .merkle_tree
            .leaves
            .iter()
            .map(|leaf| leaf.entitlements * cohort.amount_per_entitlement)
            .sum();

        // Distribute tokens across vaults
        let tokens_per_vault = total_tokens_for_cohort / cohort.vault_count as u64;
        let remainder = total_tokens_for_cohort % cohort.vault_count as u64;

        for vault_index in 0..cohort.vault_count {
            let required_tokens = if vault_index < remainder as usize {
                tokens_per_vault + 1
            } else {
                tokens_per_vault
            };

            db.insert_vault(
                &cohort.name,
                vault_index,
                cohort.vaults[vault_index],
                required_tokens,
            )
            .map_err(|e| CompilerError::InvalidConfig(format!("Failed to insert vault: {}", e)))?;
        }
    }

    Ok(())
}

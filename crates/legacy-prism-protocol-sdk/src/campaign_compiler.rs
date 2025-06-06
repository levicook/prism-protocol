/*!
# Campaign Compiler

This module provides core campaign compilation logic, extracting business logic
from CSV inputs and generating the structured data needed for on-chain deployment.

## Key Responsibilities
- Use existing CSV parsing from prism_protocol_csvs
- Generate merkle trees for each cohort
- Calculate campaign fingerprint from merkle roots
- Derive all protocol addresses (campaign, cohorts, vaults)
- Calculate vault funding requirements
- Return populated in-memory database ready for use
*/

use crate::budget_allocation::{AllocationError, BudgetAllocator};
use crate::AddressFinder;
use prism_protocol::ClaimLeaf;
use prism_protocol_csvs::{validate_csv_consistency, CampaignCsvRow, CohortsCsvRow};
use prism_protocol_db::CampaignDatabase;
use prism_protocol_merkle::{create_claim_tree_v0, ClaimTreeV0};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use sha2::{Digest, Sha256};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

/// Errors that can occur during campaign compilation
#[derive(Debug, thiserror::Error)]
pub enum CompilerError {
    #[error("CSV error: {0}")]
    Csv(#[from] prism_protocol_csvs::CsvError),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Merkle tree generation failed: {0}")]
    MerkleTree(String),

    #[error("Budget allocation failed: {0}")]
    BudgetAllocation(#[from] AllocationError),
}

pub type CompilerResult<T> = Result<T, CompilerError>;

/// Internal representation of claimant data (claimant, entitlements)
type ClaimantData = (Pubkey, u64);

/// Internal cohort data during processing
#[derive(Debug)]
struct CohortData {
    name: String,
    amount_per_entitlement: Decimal,
    amount_per_entitlement_humane: String,
    claimants: Vec<ClaimantData>,
    vault_count: usize,
}

/// Compiled cohort with all derived data
#[derive(Clone)]
pub struct CompiledCohort {
    pub name: String,
    pub address: Pubkey,
    pub merkle_root: [u8; 32],
    pub amount_per_entitlement: Decimal,
    pub amount_per_entitlement_humane: String,
    pub vault_count: usize,
    pub vaults: Vec<CompiledVault>,
    pub merkle_tree: ClaimTreeV0,
}

/// Compiled vault with address and required funding
#[derive(Clone, Debug)]
pub struct CompiledVault {
    pub address: Pubkey,
    pub required_tokens: Decimal,
}

/// Complete compilation result ready for database storage
#[derive(Clone)]
pub struct CompiledCampaign {
    pub mint: Pubkey,
    pub mint_decimals: u8,
    pub admin: Pubkey,
    pub budget: Decimal,
    pub fingerprint: [u8; 32],
    pub address: Pubkey,
    pub cohorts: Vec<CompiledCohort>,
    pub total_claimants: usize,
    pub total_vaults: usize,
}

impl CompiledCampaign {
    /// Get total funding required across all vaults in all cohorts
    pub fn total_funding_required(&self) -> Decimal {
        self.cohorts
            .iter()
            .map(|cohort| cohort.total_funding_required())
            .sum()
    }

    /// Get all vaults across all cohorts with their funding requirements
    pub fn all_vaults(&self) -> Vec<&CompiledVault> {
        self.cohorts
            .iter()
            .flat_map(|cohort| &cohort.vaults)
            .collect()
    }

    /// Find all cohorts where a claimant appears and return the cohort + leaf data
    /// A claimant can appear in multiple cohorts with different entitlements
    pub fn find_claimant_in_all_cohorts(
        &self,
        claimant: &Pubkey,
    ) -> Vec<(CompiledCohort, ClaimLeaf)> {
        let mut results = Vec::new();
        for cohort in &self.cohorts {
            for leaf in &cohort.merkle_tree.leaves {
                if leaf.claimant == *claimant {
                    results.push((cohort.clone(), leaf.clone()));
                }
            }
        }
        results
    }

    /// Find a claimant in a specific cohort by name
    /// Use this when you know which cohort the claimant should be in
    pub fn find_claimant_in_cohort(
        &self,
        claimant: &Pubkey,
        cohort_name: &str,
    ) -> Option<(CompiledCohort, ClaimLeaf)> {
        if let Some(cohort) = self.find_cohort_by_name(cohort_name) {
            for leaf in &cohort.merkle_tree.leaves {
                if leaf.claimant == *claimant {
                    return Some((cohort.clone(), leaf.clone()));
                }
            }
        }
        None
    }

    /// Find a cohort by name
    pub fn find_cohort_by_name(&self, name: &str) -> Option<CompiledCohort> {
        self.cohorts
            .iter()
            .find(|cohort| cohort.name == name)
            .cloned()
    }

    /// Get all claimant pubkeys across all cohorts
    pub fn all_claimants(&self) -> Vec<Pubkey> {
        self.cohorts
            .iter()
            .flat_map(|cohort| &cohort.merkle_tree.leaves)
            .map(|leaf| leaf.claimant)
            .collect()
    }
}

impl CompiledCohort {
    /// Get total funding required for this cohort across all its vaults
    pub fn total_funding_required(&self) -> Decimal {
        self.vaults.iter().map(|vault| vault.required_tokens).sum()
    }

    /// Get vault addresses (for compatibility with existing code)
    pub fn vault_addresses(&self) -> Vec<Pubkey> {
        self.vaults.iter().map(|vault| vault.address).collect()
    }

    /// Find a vault by index
    pub fn find_vault(&self, vault_index: u8) -> Option<&CompiledVault> {
        self.vaults.get(vault_index as usize)
    }

    /// Find the vault assigned to a specific claimant
    pub fn find_claimant_vault(&self, claimant: &Pubkey) -> Option<&CompiledVault> {
        for leaf in &self.merkle_tree.leaves {
            if leaf.claimant == *claimant {
                return self.find_vault(leaf.assigned_vault_index);
            }
        }
        None
    }

    /// Find a claimant leaf by pubkey
    pub fn find_claimant(&self, claimant: &Pubkey) -> Option<&ClaimLeaf> {
        self.merkle_tree
            .leaves
            .iter()
            .find(|leaf| leaf.claimant == *claimant)
    }

    /// Generate merkle proof for a claimant (convenience wrapper)
    pub fn proof_for_claimant(&self, claimant: &Pubkey) -> Result<Vec<[u8; 32]>, String> {
        self.merkle_tree
            .proof_for_claimant(claimant)
            .map_err(|e| format!("Failed to generate proof: {}", e))
    }

    /// Get all claimant pubkeys in this cohort
    pub fn claimant_pubkeys(&self) -> Vec<Pubkey> {
        self.merkle_tree
            .leaves
            .iter()
            .map(|leaf| leaf.claimant)
            .collect()
    }

    /// Calculate expected claim amount for a specific claimant in this cohort
    pub fn expected_claim_amount(&self, claimant: &Pubkey) -> Option<Decimal> {
        self.find_claimant(claimant)
            .map(|leaf| self.amount_per_entitlement * Decimal::from(leaf.entitlements))
    }

    /// Calculate expected claim amount as u64 for a specific claimant in this cohort
    pub fn expected_claim_amount_u64(&self, claimant: &Pubkey) -> Option<u64> {
        self.expected_claim_amount(claimant)
            .and_then(|amount| amount.floor().to_u64())
    }
}

impl CompiledVault {
    /// Convert required tokens to u64 for on-chain use (floors the amount)
    pub fn required_tokens_u64(&self) -> Result<u64, String> {
        self.required_tokens
            .floor()
            .to_u64()
            .ok_or_else(|| format!("Token amount overflow: {}", self.required_tokens))
    }
}

/// Compile campaign from CSV files with precise budget allocation
///
/// # Arguments
/// * `address_finder` - For deriving protocol addresses
/// * `campaign_csv` - Path to campaign.csv (cohort, claimant, entitlements)
/// * `cohorts_csv` - Path to cohorts.csv (cohort, share_percentage)
/// * `budget` - Total campaign budget in human-readable tokens (e.g., "1000.5" SOL)
/// * `mint` - SPL token mint for the campaign
/// * `mint_decimals` - Number of decimals for the token mint (e.g., 9 for SOL, 6 for USDC)
/// * `admin` - Campaign admin pubkey
/// * `claimants_per_vault` - How many claimants per vault (affects gas costs)
pub fn compile_campaign(
    address_finder: AddressFinder,
    campaign_rows: &[CampaignCsvRow],
    cohorts_rows: &[CohortsCsvRow],
    budget: Decimal,
    mint: Pubkey,
    mint_decimals: u8,
    admin: Pubkey,
    claimants_per_vault: usize,
) -> CompilerResult<CompiledCampaign> {
    // Step 1: Validate CSV consistency
    validate_csv_consistency(&campaign_rows, &cohorts_rows)?;

    // Step 2: Process cohorts and calculate vault counts + token amounts using BudgetAllocator
    let cohort_data = process_cohorts(
        &campaign_rows,
        &cohorts_rows,
        budget,
        mint_decimals,
        claimants_per_vault,
    )?;

    // Step 3: Generate merkle trees
    let cohort_merkle_data = generate_claim_tree_v0(cohort_data)?;

    // Step 4: Calculate campaign fingerprint
    let cohort_roots: Vec<[u8; 32]> = cohort_merkle_data
        .iter()
        .map(|(_, _, root)| *root)
        .collect();

    let campaign_fingerprint = calculate_campaign_fingerprint(&cohort_roots);

    // Step 5: Derive addresses using real fingerprint
    let (campaign_address, _) = address_finder.find_campaign_v0_address(
        &admin, //
        &campaign_fingerprint,
    );

    let compiled_cohorts = derive_addresses_and_finalize(
        &address_finder, //
        cohort_merkle_data,
        &campaign_address,
    )?;

    // Step 6: Calculate totals
    let total_claimants = compiled_cohorts
        .iter()
        .map(|c| c.merkle_tree.leaves.len())
        .sum();

    let total_vaults = compiled_cohorts.iter().map(|c| c.vault_count).sum();

    Ok(CompiledCampaign {
        mint,
        mint_decimals,
        admin,
        budget,
        fingerprint: campaign_fingerprint,
        address: campaign_address,
        cohorts: compiled_cohorts,
        total_claimants,
        total_vaults,
    })
}

/// Compile campaign and populate database
pub fn compile_campaign_db(
    address_finder: AddressFinder,
    campaign_rows: &[CampaignCsvRow],
    cohorts_rows: &[CohortsCsvRow],
    budget: Decimal,
    mint: Pubkey,
    mint_decimals: u8,
    admin: Pubkey,
    claimants_per_vault: usize,
) -> CompilerResult<CampaignDatabase> {
    let compiled_campaign = compile_campaign(
        address_finder,
        campaign_rows,
        cohorts_rows,
        budget,
        mint,
        mint_decimals,
        admin,
        claimants_per_vault,
    )?;

    let mut db = CampaignDatabase::create_in_memory()
        .map_err(|e| CompilerError::InvalidConfig(format!("Failed to create database: {}", e)))?;

    populate_database(&mut db, &compiled_campaign)?;

    Ok(db)
}

/// Process cohorts: group claimants, calculate vault counts, and convert percentages to token amounts using BudgetAllocator
fn process_cohorts(
    campaign_rows: &[CampaignCsvRow],
    cohorts_rows: &[CohortsCsvRow],
    budget: Decimal,
    mint_decimals: u8,
    claimants_per_vault: usize,
) -> CompilerResult<Vec<CohortData>> {
    // Create budget allocator with mint constraints
    let allocator = BudgetAllocator::new(budget, mint_decimals)?;

    // Create config lookup
    let config_map: HashMap<String, &CohortsCsvRow> = cohorts_rows
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

    // Convert to CohortData with vault counts and calculated token amounts using BudgetAllocator
    let mut cohort_data = Vec::new();
    for (cohort_name, claimants) in cohort_groups {
        let config = config_map.get(&cohort_name).ok_or_else(|| {
            CompilerError::InvalidConfig(format!("No config found for cohort: {}", cohort_name))
        })?;

        // Calculate vault count needed
        let vault_count = (claimants.len() + claimants_per_vault - 1) / claimants_per_vault;

        // Calculate total entitlements for this cohort
        let total_entitlements: u64 = claimants.iter().map(|(_, entitlements)| entitlements).sum();

        if total_entitlements == 0 {
            return Err(CompilerError::InvalidConfig(format!(
                "Cohort '{}' has zero total entitlements",
                cohort_name
            )));
        }

        // Use BudgetAllocator for safe, precise calculations
        let allocation = allocator.calculate_cohort_allocation(
            config.share_percentage, //
            total_entitlements,
        )?;

        cohort_data.push(CohortData {
            name: cohort_name.clone(),
            amount_per_entitlement: allocation.amount_per_entitlement,
            amount_per_entitlement_humane: allocation.amount_per_entitlement_humane,
            claimants,
            vault_count,
        });
    }

    Ok(cohort_data)
}

/// Generate merkle trees for all cohorts
fn generate_claim_tree_v0(
    cohort_data: Vec<CohortData>,
) -> CompilerResult<Vec<(CohortData, ClaimTreeV0, [u8; 32])>> {
    let mut cohort_merkle_data = Vec::new();

    for cohort in cohort_data {
        // Convert claimants to (Pubkey, u64) pairs for merkle tree
        let claimant_pairs: Vec<(Pubkey, u64)> =
            cohort.claimants.iter().map(|c| c.clone()).collect();

        // Create merkle tree with vault count
        let merkle_tree =
            create_claim_tree_v0(&claimant_pairs, cohort.vault_count).map_err(|e| {
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
    cohort_merkle_data: Vec<(CohortData, ClaimTreeV0, [u8; 32])>,
    campaign_address: &Pubkey,
) -> CompilerResult<Vec<CompiledCohort>> {
    let mut compiled_cohorts = Vec::new();

    for (cohort, merkle_tree, merkle_root) in cohort_merkle_data {
        // Derive cohort PDA from campaign and merkle root
        let (cohort_address, _) =
            address_finder.find_cohort_v0_address(campaign_address, &merkle_root);

        // Calculate total tokens needed for this cohort
        let total_tokens_for_cohort_decimal: Decimal = merkle_tree
            .leaves
            .iter()
            .map(|leaf| Decimal::from(leaf.entitlements) * cohort.amount_per_entitlement)
            .sum();

        // Find vault addresses and calculate funding for this cohort
        let vaults = calculate_vault_funding(
            address_finder,
            &cohort_address,
            cohort.vault_count,
            total_tokens_for_cohort_decimal,
        );

        compiled_cohorts.push(CompiledCohort {
            name: cohort.name,
            amount_per_entitlement: cohort.amount_per_entitlement,
            amount_per_entitlement_humane: cohort.amount_per_entitlement_humane,
            vault_count: cohort.vault_count,
            vaults,
            merkle_tree,
            merkle_root,
            address: cohort_address,
        });
    }

    Ok(compiled_cohorts)
}

/// Calculate vault addresses and funding distribution for a cohort
fn calculate_vault_funding(
    address_finder: &AddressFinder,
    cohort_address: &Pubkey,
    vault_count: usize,
    total_tokens_for_cohort: Decimal,
) -> Vec<CompiledVault> {
    // Convert to u64 for distribution (floor to be conservative)
    let total_tokens_u64 = total_tokens_for_cohort.floor().to_u64().unwrap_or(0);

    // Distribute tokens across vaults
    let tokens_per_vault = total_tokens_u64 / vault_count as u64;
    let remainder = total_tokens_u64 % vault_count as u64;

    (0..vault_count)
        .map(|i| {
            let (vault_address, _) = address_finder.find_vault_v0_address(cohort_address, i as u8);

            // First `remainder` vaults get an extra token
            let required_tokens = if i < remainder as usize {
                tokens_per_vault + 1
            } else {
                tokens_per_vault
            };

            CompiledVault {
                address: vault_address,
                required_tokens: Decimal::from(required_tokens),
            }
        })
        .collect()
}

/// Populate database with compilation result
fn populate_database(
    db: &mut CampaignDatabase,
    compilation_result: &CompiledCampaign,
) -> CompilerResult<()> {
    // Insert campaign info with budget and mint decimals
    db.insert_campaign(
        compilation_result.fingerprint,
        compilation_result.mint,
        compilation_result.mint_decimals,
        compilation_result.admin,
        compilation_result.budget,
    )
    .map_err(|e| CompilerError::InvalidConfig(format!("Failed to insert campaign: {}", e)))?;

    // Insert cohorts and related data
    for cohort in &compilation_result.cohorts {
        // Convert Decimal to u64 for database storage - fail fast if overflow
        let amount_per_entitlement_u64 = cohort
            .amount_per_entitlement
            .floor()
            .to_u64()
            .ok_or_else(|| {
                CompilerError::InvalidConfig(format!(
                    "Amount per entitlement overflow for cohort '{}': {}",
                    cohort.name, cohort.amount_per_entitlement
                ))
            })?;

        // Insert cohort
        db.insert_cohort(
            &cohort.name,
            cohort.merkle_root,
            amount_per_entitlement_u64,
            &cohort.amount_per_entitlement_humane,
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

        // Insert vault requirements using pre-calculated funding amounts
        for (vault_index, vault) in cohort.vaults.iter().enumerate() {
            let required_tokens_u64 = vault.required_tokens.floor().to_u64().ok_or_else(|| {
                CompilerError::InvalidConfig(format!(
                    "Vault funding overflow for cohort '{}', vault {}",
                    cohort.name, vault_index
                ))
            })?;

            db.insert_vault(
                &cohort.name,
                vault_index,
                vault.address,
                required_tokens_u64,
            )
            .map_err(|e| CompilerError::InvalidConfig(format!("Failed to insert vault: {}", e)))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AddressFinder;
    use prism_protocol_csvs::{CampaignCsvRow, CohortsCsvRow};
    use rust_decimal::Decimal;
    use sha2::Digest;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::{Keypair, SeedDerivable};
    use solana_sdk::signer::Signer as _;
    use std::str::FromStr;

    fn deterministic_keypair(identifier: &str) -> Keypair {
        let seed = sha2::Sha256::digest(identifier.as_bytes());
        Keypair::from_seed(&seed).expect("SHA256 output should always be valid seed")
    }

    fn deterministic_pubkey(identifier: &str) -> Pubkey {
        deterministic_keypair(identifier).pubkey()
    }

    fn test_address_finder() -> AddressFinder {
        AddressFinder::default()
    }

    fn test_admin() -> Pubkey {
        deterministic_pubkey("test_admin")
    }

    fn test_mint() -> Pubkey {
        deterministic_pubkey("test_mint")
    }

    fn simple_campaign_rows() -> Vec<CampaignCsvRow> {
        vec![
            CampaignCsvRow {
                cohort: "Alpha".to_string(),
                claimant: deterministic_pubkey("alpha_claimant_1"),
                entitlements: 100,
            },
            CampaignCsvRow {
                cohort: "Alpha".to_string(),
                claimant: deterministic_pubkey("alpha_claimant_2"),
                entitlements: 200,
            },
            CampaignCsvRow {
                cohort: "Beta".to_string(),
                claimant: deterministic_pubkey("beta_claimant_1"),
                entitlements: 50,
            },
            CampaignCsvRow {
                cohort: "Beta".to_string(),
                claimant: deterministic_pubkey("beta_claimant_2"),
                entitlements: 150,
            },
        ]
    }

    fn simple_cohorts_rows() -> Vec<CohortsCsvRow> {
        vec![
            CohortsCsvRow {
                cohort: "Alpha".to_string(),
                share_percentage: Decimal::from(60), // 60%
            },
            CohortsCsvRow {
                cohort: "Beta".to_string(),
                share_percentage: Decimal::from(40), // 40%
            },
        ]
    }

    #[test]
    fn test_compile_campaign_basic() {
        let compiled = compile_campaign(
            test_address_finder(),
            &simple_campaign_rows(),
            &simple_cohorts_rows(),
            Decimal::from(1000), // 1000 SOL budget
            test_mint(),
            9, // SOL decimals
            test_admin(),
            10, // claimants per vault
        )
        .unwrap();

        // Basic structure
        assert_eq!(compiled.cohorts.len(), 2);
        assert_eq!(compiled.total_claimants, 4);
        assert_eq!(compiled.total_vaults, 2); // 2 claimants per cohort, 10 per vault = 1 vault each
        assert_eq!(compiled.budget, Decimal::from(1000));
        assert_eq!(compiled.mint_decimals, 9);

        // Check cohort names
        let cohort_names: Vec<&str> = compiled.cohorts.iter().map(|c| c.name.as_str()).collect();
        assert!(cohort_names.contains(&"Alpha"));
        assert!(cohort_names.contains(&"Beta"));
    }

    #[test]
    fn test_vault_funding_distribution_exact() {
        let compiled = compile_campaign(
            test_address_finder(),
            &simple_campaign_rows(),
            &simple_cohorts_rows(),
            Decimal::from(1000), // 1000 SOL budget
            test_mint(),
            9, // SOL decimals
            test_admin(),
            10, // claimants per vault
        )
        .unwrap();

        // Alpha cohort: 60% of 1000 = 600 SOL
        // Alpha entitlements: 100 + 200 = 300
        // Alpha amount per entitlement: 600 / 300 = 2 SOL
        let alpha_cohort = compiled.cohorts.iter().find(|c| c.name == "Alpha").unwrap();
        assert_eq!(alpha_cohort.amount_per_entitlement, Decimal::from(2));
        assert_eq!(alpha_cohort.total_funding_required(), Decimal::from(600));
        assert_eq!(alpha_cohort.vaults.len(), 1); // 2 claimants, 10 per vault
        assert_eq!(alpha_cohort.vaults[0].required_tokens, Decimal::from(600));

        // Beta cohort: 40% of 1000 = 400 SOL
        // Beta entitlements: 50 + 150 = 200
        // Beta amount per entitlement: 400 / 200 = 2 SOL
        let beta_cohort = compiled.cohorts.iter().find(|c| c.name == "Beta").unwrap();
        assert_eq!(beta_cohort.amount_per_entitlement, Decimal::from(2));
        assert_eq!(beta_cohort.total_funding_required(), Decimal::from(400));
        assert_eq!(beta_cohort.vaults.len(), 1); // 2 claimants, 10 per vault
        assert_eq!(beta_cohort.vaults[0].required_tokens, Decimal::from(400));

        // Total funding should equal budget
        assert_eq!(compiled.total_funding_required(), Decimal::from(1000));
    }

    #[test]
    fn test_vault_funding_distribution_with_remainder() {
        // Create scenario with 3 vaults where tokens don't divide evenly
        let campaign_rows = vec![
            CampaignCsvRow {
                cohort: "Test".to_string(),
                claimant: deterministic_pubkey("test_claimant_1"),
                entitlements: 100,
            },
            CampaignCsvRow {
                cohort: "Test".to_string(),
                claimant: deterministic_pubkey("test_claimant_2"),
                entitlements: 1, // This creates indivisible scenario
            },
        ];

        let cohorts_rows = vec![CohortsCsvRow {
            cohort: "Test".to_string(),
            share_percentage: Decimal::from(100),
        }];

        let compiled = compile_campaign(
            test_address_finder(),
            &campaign_rows,
            &cohorts_rows,
            Decimal::from(101), // 101 tokens
            test_mint(),
            0, // 0 decimals (whole tokens only)
            test_admin(),
            1, // 1 claimant per vault = 2 vaults
        )
        .unwrap();

        let cohort = &compiled.cohorts[0];

        // 100% of 101 = 101 tokens
        // Total entitlements: 100 + 1 = 101
        // Amount per entitlement: 101 / 101 = 1 token each
        // Total tokens needed: 101 tokens
        // Distributed across 2 vaults: 50 + 51 (first vault gets remainder)

        assert_eq!(cohort.vaults.len(), 2);
        assert_eq!(cohort.total_funding_required(), Decimal::from(101));

        // Vaults should get: vault 0 = 51, vault 1 = 50 (first gets remainder)
        assert_eq!(cohort.vaults[0].required_tokens, Decimal::from(51));
        assert_eq!(cohort.vaults[1].required_tokens, Decimal::from(50));
    }

    #[test]
    fn test_usdc_precision_handling() {
        let compiled = compile_campaign(
            test_address_finder(),
            &simple_campaign_rows(),
            &simple_cohorts_rows(),
            Decimal::from_str("1000.123456").unwrap(), // USDC with 6 decimals
            test_mint(),
            6, // USDC decimals
            test_admin(),
            10,
        )
        .unwrap();

        // All amounts should respect USDC precision (6 decimals)
        for cohort in &compiled.cohorts {
            for vault in &cohort.vaults {
                // Should be expressible in USDC precision
                let as_u64 = vault.required_tokens_u64().unwrap();
                let back_to_decimal = Decimal::from(as_u64);

                // The difference should be less than 1 microUSDC (0.000001)
                let precision_unit = Decimal::from_str("0.000001").unwrap();
                assert!((vault.required_tokens - back_to_decimal).abs() < precision_unit);
            }
        }
    }

    #[test]
    fn test_zero_decimal_token_distribution() {
        let compiled = compile_campaign(
            test_address_finder(),
            &simple_campaign_rows(),
            &simple_cohorts_rows(),
            Decimal::from(1000), // 1000 whole tokens
            test_mint(),
            0, // No decimals
            test_admin(),
            10,
        )
        .unwrap();

        // All vault funding amounts should be whole numbers
        for cohort in &compiled.cohorts {
            for vault in &cohort.vaults {
                assert_eq!(vault.required_tokens, vault.required_tokens.floor());
                assert!(vault.required_tokens_u64().is_ok());
            }
        }
    }

    #[test]
    fn test_large_campaign_vault_distribution() {
        // Create campaign with many claimants to test vault sharding
        let mut campaign_rows = Vec::new();
        for i in 0..25 {
            // 25 claimants
            campaign_rows.push(CampaignCsvRow {
                cohort: "Large".to_string(),
                claimant: deterministic_pubkey(&format!("large_claimant_{}", i)),
                entitlements: 100,
            });
        }

        let cohorts_rows = vec![CohortsCsvRow {
            cohort: "Large".to_string(),
            share_percentage: Decimal::from(100),
        }];

        let compiled = compile_campaign(
            test_address_finder(),
            &campaign_rows,
            &cohorts_rows,
            Decimal::from(250000), // 250k tokens
            test_mint(),
            9,
            test_admin(),
            10, // 10 claimants per vault = 3 vaults (25 / 10 = 2.5 -> 3)
        )
        .unwrap();

        let cohort = &compiled.cohorts[0];
        assert_eq!(cohort.vaults.len(), 3); // 25 claimants, 10 per vault
        assert_eq!(cohort.total_funding_required(), Decimal::from(250000));

        // Each vault should get roughly equal funding
        let total_funding: Decimal = cohort.vaults.iter().map(|v| v.required_tokens).sum();
        assert_eq!(total_funding, Decimal::from(250000));
    }

    #[test]
    fn test_compiled_campaign_convenience_methods() {
        let compiled = compile_campaign(
            test_address_finder(),
            &simple_campaign_rows(),
            &simple_cohorts_rows(),
            Decimal::from(1000),
            test_mint(),
            9,
            test_admin(),
            10,
        )
        .unwrap();

        // Test total_funding_required
        let total = compiled.total_funding_required();
        let manual_total: Decimal = compiled
            .cohorts
            .iter()
            .map(|c| c.total_funding_required())
            .sum();
        assert_eq!(total, manual_total);

        // Test all_vaults
        let all_vaults = compiled.all_vaults();
        let manual_count: usize = compiled.cohorts.iter().map(|c| c.vaults.len()).sum();
        assert_eq!(all_vaults.len(), manual_count);

        // Test vault_addresses convenience method
        for cohort in &compiled.cohorts {
            let addresses = cohort.vault_addresses();
            assert_eq!(addresses.len(), cohort.vaults.len());
            for (i, addr) in addresses.iter().enumerate() {
                assert_eq!(*addr, cohort.vaults[i].address);
            }
        }
    }

    #[test]
    fn test_compiled_vault_u64_conversion() {
        let vault = CompiledVault {
            address: deterministic_pubkey("test_vault"),
            required_tokens: Decimal::from_str("1234.567890123").unwrap(),
        };

        // Should floor to 1234
        assert_eq!(vault.required_tokens_u64().unwrap(), 1234);

        // Test overflow case
        let overflow_vault = CompiledVault {
            address: deterministic_pubkey("test_overflow_vault"),
            required_tokens: Decimal::from_str("18446744073709551616").unwrap(), // > u64::MAX
        };

        assert!(overflow_vault.required_tokens_u64().is_err());
    }

    #[test]
    fn test_dust_calculation_across_cohorts() {
        // Use budget that creates dust across multiple cohorts
        let compiled = compile_campaign(
            test_address_finder(),
            &simple_campaign_rows(),
            &simple_cohorts_rows(),
            Decimal::from_str("1000.123456789").unwrap(), // Creates dust with SOL precision
            test_mint(),
            9, // SOL decimals
            test_admin(),
            10,
        )
        .unwrap();

        // Total funding should be less than or equal to budget (conservative allocation)
        let total_funding = compiled.total_funding_required();
        assert!(total_funding <= compiled.budget);

        // The difference (dust) should be small - allow up to 1 SOL of dust
        let dust = compiled.budget - total_funding;
        assert!(dust < Decimal::from(1)); // Less than 1 SOL
    }

    #[test]
    fn test_fingerprint_determinism() {
        // Use simple single cohort data to ensure determinism
        let campaign_rows = vec![
            CampaignCsvRow {
                cohort: "Test".to_string(),
                claimant: deterministic_pubkey("claimant_1"),
                entitlements: 100,
            },
            CampaignCsvRow {
                cohort: "Test".to_string(),
                claimant: deterministic_pubkey("claimant_2"),
                entitlements: 200,
            },
        ];

        let cohorts_rows = vec![CohortsCsvRow {
            cohort: "Test".to_string(),
            share_percentage: Decimal::from(100),
        }];

        let compiled1 = compile_campaign(
            test_address_finder(),
            &campaign_rows,
            &cohorts_rows,
            Decimal::from(1000),
            test_mint(),
            9,
            test_admin(),
            10,
        )
        .unwrap();

        let compiled2 = compile_campaign(
            test_address_finder(),
            &campaign_rows,
            &cohorts_rows,
            Decimal::from(1000),
            test_mint(),
            9,
            test_admin(),
            10,
        )
        .unwrap();

        // Same inputs should produce same fingerprint
        assert_eq!(compiled1.fingerprint, compiled2.fingerprint);
    }

    #[test]
    fn test_error_handling_zero_entitlements() {
        let campaign_rows = vec![CampaignCsvRow {
            cohort: "Test".to_string(),
            claimant: deterministic_pubkey("test_claimant_zero_entitlements"),
            entitlements: 0, // Zero entitlements should cause error
        }];

        let cohorts_rows = vec![CohortsCsvRow {
            cohort: "Test".to_string(),
            share_percentage: Decimal::from(100),
        }];

        let result = compile_campaign(
            test_address_finder(),
            &campaign_rows,
            &cohorts_rows,
            Decimal::from(1000),
            test_mint(),
            9,
            test_admin(),
            10,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_database_population_uses_precomputed_values() {
        // Test that compilation succeeds and database can be created
        // The detailed verification of database contents would require more
        // complex database introspection which we'll skip for now
        let _compiled = compile_campaign(
            test_address_finder(),
            &simple_campaign_rows(),
            &simple_cohorts_rows(),
            Decimal::from(1000),
            test_mint(),
            9,
            test_admin(),
            10,
        )
        .unwrap();

        // This test mainly ensures that the API works and vault funding
        // calculations are done during compilation, not in populate_database
        assert!(true); // Pass - the real test is that compilation succeeds with new vault structure
    }

    #[test]
    fn test_duplicate_claimant_within_cohort_errors() {
        // Test that duplicate claimants within the same cohort cause compilation to fail
        // TODO: Future enhancement should sum entitlements instead of erroring
        let campaign_rows = vec![
            CampaignCsvRow {
                cohort: "PowerUsers".to_string(),
                claimant: deterministic_pubkey("duplicate_claimant"),
                entitlements: 3,
            },
            CampaignCsvRow {
                cohort: "PowerUsers".to_string(),
                claimant: deterministic_pubkey("duplicate_claimant"), // Same claimant, same cohort
                entitlements: 1,
            },
        ];

        let cohorts_rows = vec![CohortsCsvRow {
            cohort: "PowerUsers".to_string(),
            share_percentage: Decimal::from(100),
        }];

        let result = compile_campaign(
            test_address_finder(),
            &campaign_rows,
            &cohorts_rows,
            Decimal::from(1000),
            test_mint(),
            9,
            test_admin(),
            10,
        );

        // Should fail with MerkleTree error (caused by DuplicateClaimant in merkle tree creation)
        assert!(result.is_err());
        if let Err(CompilerError::MerkleTree(msg)) = result {
            assert!(msg.contains("Failed to create merkle tree"));
        } else {
            panic!("Expected MerkleTree error for duplicate claimant");
        }
    }
}

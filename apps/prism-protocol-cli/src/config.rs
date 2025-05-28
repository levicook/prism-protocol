use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::path::PathBuf;

/// Campaign configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignConfig {
    /// Human-readable campaign name (for organization)
    pub campaign_name: String,

    /// SPL token mint to be distributed
    pub mint: Pubkey,

    /// Path to admin keypair file
    pub admin_keypair_path: PathBuf,

    /// Optional claim deadline (Unix timestamp)
    pub claim_deadline_timestamp: Option<i64>,

    /// List of cohorts in this campaign
    pub cohorts: Vec<CohortConfig>,
}

/// Configuration for a single cohort within a campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortConfig {
    /// Human-readable cohort name (for organization)
    pub cohort_name: String,

    /// Amount of tokens per entitlement
    pub amount_per_entitlement: u64,

    /// Path to claimants file (CSV or JSON)
    pub claimants_file: PathBuf,

    /// Number of vaults to use for this cohort
    #[serde(default = "default_vault_count")]
    pub vault_count: usize,

    /// Optional: Pre-specified vault pubkeys (if not provided, will be generated)
    pub vaults: Option<Vec<Pubkey>>,
}

/// Claimant data structure for input files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimantInput {
    /// Claimant's public key
    pub claimant: Pubkey,

    /// Number of entitlements for this claimant
    #[serde(default = "default_entitlements")]
    pub entitlements: u64,
}

/// Generated campaign output structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignOutput {
    /// The calculated campaign fingerprint
    pub campaign_fingerprint: String,

    /// Campaign initialization parameters
    pub campaign_params: CampaignParams,

    /// Cohort data for each cohort
    pub cohorts: Vec<CohortOutput>,

    /// Summary information
    pub summary: CampaignSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignParams {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub campaign_fingerprint: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortOutput {
    /// Human-readable cohort name
    pub cohort_name: String,

    /// Merkle root for this cohort
    pub merkle_root: String,

    /// Cohort initialization parameters
    pub cohort_params: CohortParams,

    /// Vault funding requirements
    pub vault_funding: Vec<VaultFunding>,

    /// Path to claimant lookup file
    pub claimant_lookup_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortParams {
    pub campaign_fingerprint: [u8; 32],
    pub merkle_root: [u8; 32],
    pub amount_per_entitlement: u64,
    pub vaults: Vec<Pubkey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultFunding {
    pub vault: Pubkey,
    pub required_amount: u64,
    pub claimant_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSummary {
    pub total_claimants: usize,
    pub total_cohorts: usize,
    pub total_tokens_required: u64,
    pub total_vaults: usize,
}

/// Claimant lookup data (output for frontend integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimantLookup {
    pub claimant: Pubkey,
    pub cohort_name: String,
    pub merkle_root: String,
    pub merkle_proof: Vec<String>,
    pub assigned_vault: Pubkey,
    pub entitlements: u64,
    pub amount_per_entitlement: u64,
    pub total_claimable: u64,
}

fn default_vault_count() -> usize {
    5
}

fn default_entitlements() -> u64 {
    1
}

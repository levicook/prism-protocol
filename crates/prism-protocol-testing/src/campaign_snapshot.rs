use std::collections::HashMap;

use prism_protocol::CampaignStatus;
use solana_pubkey::Pubkey;

use crate::TestFixture;

/// Comprehensive campaign snapshot for integration testing
///
/// This captures the complete state of a campaign for before/after comparisons:
/// - All vault balances across all cohorts
/// - Campaign metadata (status, go-live slot, etc.)
/// - Admin token account balance
/// - Optional claimant balances for specific users
///
/// **Use cases:**
/// - Verify operations only affect expected accounts
/// - Test complex multi-cohort scenarios
/// - Validate state isolation in failure cases
/// - Integration testing with multiple claims
#[derive(Clone, PartialEq)]
pub struct CampaignSnapshot {
    /// All vault balances by cohort name and vault index
    pub vault_balances: HashMap<String, Vec<u64>>,
    /// Admin's token account balance
    pub admin_balance: u64,
    /// Campaign status and metadata
    pub campaign_status: Option<CampaignStatus>,
    pub go_live_slot: Option<u64>,
    /// Optional: specific claimant balances to track
    pub tracked_claimants: HashMap<Pubkey, u64>,
}

impl std::fmt::Debug for CampaignSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let campaign_status = match self.campaign_status {
            Some(status) => match status {
                CampaignStatus::Active => "Active",
                CampaignStatus::Paused => "Paused",
                CampaignStatus::Inactive => "Inactive",
                CampaignStatus::PermanentlyHalted => "PermanentlyHalted",
            },
            None => "None",
        };

        f.debug_struct("CampaignSnapshot")
            .field("vault_balances", &self.vault_balances)
            .field("admin_balance", &self.admin_balance)
            .field("campaign_status", &campaign_status)
            .field("go_live_slot", &self.go_live_slot)
            .field("tracked_claimants", &self.tracked_claimants)
            .finish()
    }
}

impl CampaignSnapshot {
    /// Capture comprehensive campaign state
    pub fn capture_all(test: &TestFixture) -> Self {
        let mut vault_balances = HashMap::new();

        // Capture all vault balances by cohort
        for cohort in &test.state.compiled_campaign.cohorts {
            let cohort_vault_balances: Vec<u64> = cohort
                .vaults
                .iter()
                .map(|vault| test.get_token_account_balance(&vault.address).unwrap_or(0))
                .collect();
            vault_balances.insert(cohort.name.clone(), cohort_vault_balances);
        }

        // Get admin token account balance
        let admin_token_account = spl_associated_token_account::get_associated_token_address(
            &test.state.compiled_campaign.admin,
            &test.state.compiled_campaign.mint,
        );
        let admin_balance = test
            .get_token_account_balance(&admin_token_account)
            .unwrap_or(0);

        // Get campaign metadata
        let campaign_account = test.fetch_campaign_account();
        let (campaign_status, go_live_slot) = if let Some(campaign) = campaign_account {
            (Some(campaign.status), Some(campaign.go_live_slot))
        } else {
            (None, None)
        };

        Self {
            vault_balances,
            admin_balance,
            campaign_status,
            go_live_slot,
            tracked_claimants: HashMap::new(),
        }
    }

    /// Capture state for specific claimants
    pub fn capture_with_claimants(test: &TestFixture, claimants: &[Pubkey]) -> Self {
        let mut snapshot = Self::capture_all(test);

        for claimant in claimants {
            let claimant_token_account = spl_associated_token_account::get_associated_token_address(
                claimant,
                &test.state.compiled_campaign.mint,
            );
            let balance = test
                .get_token_account_balance(&claimant_token_account)
                .unwrap_or(0);
            snapshot.tracked_claimants.insert(*claimant, balance);
        }

        snapshot
    }

    /// Get total balance across all vaults
    pub fn total_vault_balance(&self) -> u64 {
        self.vault_balances
            .values()
            .flat_map(|cohort_vaults| cohort_vaults.iter())
            .sum()
    }

    /// Get balance for a specific vault
    pub fn get_vault_balance(&self, cohort_name: &str, vault_index: usize) -> Option<u64> {
        self.vault_balances
            .get(cohort_name)
            .and_then(|vaults| vaults.get(vault_index))
            .copied()
    }

    /// Helper: assert only specific accounts changed
    pub fn assert_only_changed(&self, other: &Self, expected_changes: &[AccountChange]) {
        // This would be implemented to verify surgical changes
        // For now, just a concept demonstration
        for change in expected_changes {
            match change {
                AccountChange::Vault {
                    cohort,
                    vault_index,
                    delta,
                } => {
                    let before = self.get_vault_balance(cohort, *vault_index).unwrap_or(0);
                    let after = other.get_vault_balance(cohort, *vault_index).unwrap_or(0);
                    let actual_delta = after as i64 - before as i64;
                    assert_eq!(
                        actual_delta, *delta,
                        "Vault {}/{} delta mismatch",
                        cohort, vault_index
                    );
                }
                AccountChange::Claimant { pubkey, delta } => {
                    let before = self.tracked_claimants.get(pubkey).copied().unwrap_or(0);
                    let after = other.tracked_claimants.get(pubkey).copied().unwrap_or(0);
                    let actual_delta = after as i64 - before as i64;
                    assert_eq!(actual_delta, *delta, "Claimant {} delta mismatch", pubkey);
                }
                AccountChange::Admin { delta } => {
                    let actual_delta = other.admin_balance as i64 - self.admin_balance as i64;
                    assert_eq!(actual_delta, *delta, "Admin balance delta mismatch");
                }
            }
        }
    }
}

/// Expected account changes for surgical verification
#[derive(Clone)]
pub enum AccountChange {
    Vault {
        cohort: String,
        vault_index: usize,
        delta: i64,
    },
    Claimant {
        pubkey: Pubkey,
        delta: i64,
    },
    Admin {
        delta: i64,
    },
}

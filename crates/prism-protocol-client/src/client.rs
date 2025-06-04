/*!
# Prism Protocol Client Implementation

Main client providing unified access to Prism Protocol operations with proper versioning.
*/

use std::sync::Arc;

use crate::{
    errors::{ClientError, ClientResult},
    types::{SimulationResult, TransactionResult},
};
use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use prism_protocol_sdk::AddressFinder;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig},
};
use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::Transaction};

// Re-export the actual program types via SDK (with versioning)
pub use prism_protocol_sdk::{CampaignV0, ClaimReceiptV0, CohortV0};

// Re-export anchor_spl types for external use
pub use anchor_spl::token::{Mint as MintAccount, TokenAccount as TokenAccountInfo};

/// Unified client for Prism Protocol RPC operations
#[derive(Clone)]
pub struct PrismProtocolClient {
    address_finder: Arc<AddressFinder>,
    rpc_client: Arc<RpcClient>,
}

impl PrismProtocolClient {
    /// Create new client with RpcClient
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self::new_with_address_finder(rpc_client, AddressFinder::default())
    }

    /// Create new client with custom AddressFinder (for advanced usage)
    pub fn new_with_address_finder(
        rpc_client: Arc<RpcClient>,
        address_finder: AddressFinder,
    ) -> Self {
        Self {
            rpc_client,
            address_finder: Arc::new(address_finder),
        }
    }
    // ================================================================================================
    // Protocol Account Operations (Versioned) - Now much cleaner!
    // ================================================================================================

    /// Get campaign account (V0)
    pub fn get_campaign_v0(
        &self,
        fingerprint: &[u8; 32],
        admin: &Pubkey,
    ) -> ClientResult<Option<CampaignV0>> {
        let (campaign_pda, _) = self
            .address_finder
            .find_campaign_v0_address(admin, fingerprint);

        self.fetch_account(&campaign_pda)
    }

    /// Get cohort account (V0)
    pub fn get_cohort_v0(
        &self,
        campaign: &Pubkey,
        merkle_root: &[u8; 32],
    ) -> ClientResult<Option<CohortV0>> {
        let (cohort_pda, _) = self
            .address_finder
            .find_cohort_v0_address(campaign, merkle_root);

        self.fetch_account(&cohort_pda)
    }

    /// Get claim receipt account (V0) - Note: requires cohort address, not campaign
    pub fn get_claim_receipt_v0(
        &self,
        cohort: &Pubkey,
        claimant: &Pubkey,
    ) -> ClientResult<Option<ClaimReceiptV0>> {
        let (receipt_pda, _) = self
            .address_finder
            .find_claim_receipt_v0_address(cohort, claimant);

        self.fetch_account(&receipt_pda)
    }

    /// Get vault token account info - Note: requires cohort address and vault index
    pub fn get_vault_v0(
        &self,
        cohort: &Pubkey,
        vault_index: u8,
    ) -> ClientResult<Option<TokenAccount>> {
        let (vault_pda, _) = self
            .address_finder
            .find_vault_v0_address(cohort, vault_index);
        self.get_token_account(&vault_pda)
    }

    // ================================================================================================
    // SPL Token Operations (Using anchor_spl types)
    // ================================================================================================

    /// Get mint account using anchor_spl types
    pub fn get_mint(&self, mint: &Pubkey) -> ClientResult<Option<Mint>> {
        self.fetch_unchecked_account(mint)
    }

    /// Get token account using anchor_spl types
    pub fn get_token_account(&self, address: &Pubkey) -> ClientResult<Option<TokenAccount>> {
        self.fetch_unchecked_account(address)
    }

    /// Convenience method to check if a mint is WSOL
    pub fn is_wsol_mint(&self, mint: &Pubkey) -> bool {
        *mint == spl_token::native_mint::id()
    }

    /// Format token amount with proper decimals (for display)
    pub fn format_token_amount(&self, base_units: u64, decimals: u8) -> String {
        let divisor = 10_u64.pow(decimals as u32);
        let whole_tokens = base_units / divisor;
        let fractional_units = base_units % divisor;

        if fractional_units == 0 {
            format!("{}", whole_tokens)
        } else {
            // Format with trailing zeros removed
            let fractional_str = format!("{:0width$}", fractional_units, width = decimals as usize);
            let trimmed = fractional_str.trim_end_matches('0');
            if trimmed.is_empty() {
                format!("{}", whole_tokens)
            } else {
                format!("{}.{}", whole_tokens, trimmed)
            }
        }
    }

    /// Get vault balance with proper error handling
    pub fn get_vault_balance(&self, cohort: &Pubkey, vault_index: u8) -> ClientResult<u64> {
        match self.get_vault_v0(cohort, vault_index)? {
            Some(vault_account) => Ok(vault_account.amount),
            None => Ok(0), // Vault doesn't exist = 0 balance
        }
    }

    /// Get or create associated token account address
    pub fn get_associated_token_account_address(&self, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        spl_associated_token_account::get_associated_token_address(owner, mint)
    }

    // ================================================================================================
    // Transaction Management (Simulation + Execution + Logging)
    // ================================================================================================

    /// Simulate transaction without executing
    pub fn simulate_transaction(&self, tx: &Transaction) -> ClientResult<SimulationResult> {
        let config = RpcSimulateTransactionConfig {
            sig_verify: true,
            replace_recent_blockhash: true,
            commitment: Some(self.rpc_client.commitment()),
            encoding: None,
            accounts: None,
            min_context_slot: None,
            inner_instructions: false,
        };

        let result = self
            .rpc_client
            .simulate_transaction_with_config(tx, config)?;
        Ok(SimulationResult::from_rpc_result(result.value))
    }

    /// Send transaction and return signature
    pub fn send_transaction(&self, tx: &Transaction) -> ClientResult<Signature> {
        let config = RpcSendTransactionConfig {
            skip_preflight: false,
            preflight_commitment: Some(self.rpc_client.commitment().commitment),
            encoding: None,
            max_retries: Some(3),
            min_context_slot: None,
        };

        let signature = self.rpc_client.send_transaction_with_config(tx, config)?;

        // Generate explorer URL for easy debugging
        println!(
            "✅ Transaction: https://explorer.solana.com/tx/{}",
            signature
        );

        Ok(signature)
    }

    /// Simulate and optionally send transaction (supports dry-run)
    pub fn simulate_and_send(
        &self,
        tx: &Transaction,
        dry_run: bool,
    ) -> ClientResult<TransactionResult> {
        // Always simulate first
        let sim_result = self.simulate_transaction(tx)?;

        if !sim_result.success {
            return Err(ClientError::SimulationFailed(format!(
                "Transaction simulation failed: {}",
                sim_result
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string())
            )));
        }

        if dry_run {
            println!(
                "🧪 Dry run successful - transaction would consume {} compute units",
                sim_result.compute_units.unwrap_or(0)
            );
            return Ok(TransactionResult::Simulated(sim_result.raw));
        }

        // Execute transaction
        let signature = self.send_transaction(tx)?;
        Ok(TransactionResult::Executed(signature))
    }

    // ================================================================================================
    // Utility Methods
    // ================================================================================================

    /// Get the address finder
    pub fn address_finder(&self) -> &AddressFinder {
        &self.address_finder
    }

    /// Get the program ID
    pub fn program_id(&self) -> &Pubkey {
        &self.address_finder.program_id
    }

    /// Helper method to fetch and deserialize any account (RPC errors bubble up, deserialization errors become None)
    fn fetch_account<T>(&self, address: &Pubkey) -> ClientResult<Option<T>>
    where
        T: AccountDeserialize,
    {
        let account_data = self.rpc_client.get_account_data(address)?;
        Ok(T::try_deserialize(&mut &account_data[..]).ok())
    }

    /// Helper method for SPL token accounts (no discriminators)
    fn fetch_unchecked_account<T>(&self, address: &Pubkey) -> ClientResult<Option<T>>
    where
        T: AccountDeserialize,
    {
        let account_data = self.rpc_client.get_account_data(address)?;
        Ok(T::try_deserialize_unchecked(&mut &account_data[..]).ok())
    }
}

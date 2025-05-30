/*!
# Prism Protocol Client Implementation

Main client providing unified access to Prism Protocol operations with proper versioning.
*/

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
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature,
    transaction::Transaction,
};

// Re-export the actual program types via SDK (with versioning)
pub use prism_protocol_sdk::{CampaignV0, ClaimReceiptV0, CohortV0};

// Re-export anchor_spl types for external use
pub use anchor_spl::token::{Mint as MintAccount, TokenAccount as TokenAccountInfo};

/// Unified client for Prism Protocol RPC operations
pub struct PrismProtocolClient {
    address_finder: AddressFinder,
    rpc_client: RpcClient,
}

impl PrismProtocolClient {
    /// Create new client with default commitment (confirmed)
    pub fn new(rpc_url: String) -> ClientResult<Self> {
        Self::new_with_address_finder_and_commitment(
            rpc_url,
            AddressFinder::default(),
            CommitmentConfig::confirmed(),
        )
    }

    /// Create new client with specific commitment level
    pub fn new_with_address_finder_and_commitment(
        rpc_url: String,
        address_finder: AddressFinder,
        commitment: CommitmentConfig,
    ) -> ClientResult<Self> {
        let rpc_client = RpcClient::new_with_commitment(rpc_url, commitment);

        Ok(Self {
            rpc_client,
            address_finder,
        })
    }

    // ================================================================================================
    // Protocol Account Operations (Versioned)
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

        let account_data = match self.rpc_client.get_account_data(&campaign_pda) {
            Ok(data) => data,
            Err(solana_client::client_error::ClientError {
                kind:
                    solana_client::client_error::ClientErrorKind::RpcError(
                        solana_client::rpc_request::RpcError::RpcResponseError { .. },
                    ),
                ..
            }) => return Ok(None), // Account doesn't exist
            Err(e) => return Err(ClientError::Rpc(e)),
        };

        // Skip discriminator (first 8 bytes)
        if account_data.len() < 8 {
            return Err(ClientError::InvalidAccountData(
                "Account data too short for discriminator".to_string(),
            ));
        }

        let campaign: CampaignV0 = AccountDeserialize::try_deserialize(&mut &account_data[8..])
            .map_err(|e| {
                ClientError::InvalidAccountData(format!("Failed to deserialize campaign: {}", e))
            })?;

        Ok(Some(campaign))
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

        let account_data = match self.rpc_client.get_account_data(&cohort_pda) {
            Ok(data) => data,
            Err(solana_client::client_error::ClientError {
                kind:
                    solana_client::client_error::ClientErrorKind::RpcError(
                        solana_client::rpc_request::RpcError::RpcResponseError { .. },
                    ),
                ..
            }) => return Ok(None),
            Err(e) => return Err(ClientError::Rpc(e)),
        };

        if account_data.len() < 8 {
            return Err(ClientError::InvalidAccountData(
                "Account data too short for discriminator".to_string(),
            ));
        }

        let cohort: CohortV0 = AccountDeserialize::try_deserialize(&mut &account_data[8..])
            .map_err(|e| {
                ClientError::InvalidAccountData(format!("Failed to deserialize cohort: {}", e))
            })?;

        Ok(Some(cohort))
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

        let account_data = match self.rpc_client.get_account_data(&receipt_pda) {
            Ok(data) => data,
            Err(solana_client::client_error::ClientError {
                kind:
                    solana_client::client_error::ClientErrorKind::RpcError(
                        solana_client::rpc_request::RpcError::RpcResponseError { .. },
                    ),
                ..
            }) => return Ok(None),
            Err(e) => return Err(ClientError::Rpc(e)),
        };

        if account_data.len() < 8 {
            return Err(ClientError::InvalidAccountData(
                "Account data too short for discriminator".to_string(),
            ));
        }

        let receipt: ClaimReceiptV0 = AccountDeserialize::try_deserialize(&mut &account_data[8..])
            .map_err(|e| {
                ClientError::InvalidAccountData(format!(
                    "Failed to deserialize claim receipt: {}",
                    e
                ))
            })?;

        Ok(Some(receipt))
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
        let account_data = match self.rpc_client.get_account_data(mint) {
            Ok(data) => data,
            Err(solana_client::client_error::ClientError {
                kind:
                    solana_client::client_error::ClientErrorKind::RpcError(
                        solana_client::rpc_request::RpcError::RpcResponseError { .. },
                    ),
                ..
            }) => return Ok(None),
            Err(e) => return Err(ClientError::Rpc(e)),
        };

        let mint_account: Mint =
            AccountDeserialize::try_deserialize_unchecked(&mut &account_data[..])
                .map_err(|e| ClientError::SplToken(format!("Failed to deserialize mint: {}", e)))?;

        Ok(Some(mint_account))
    }

    /// Get token account using anchor_spl types
    pub fn get_token_account(&self, address: &Pubkey) -> ClientResult<Option<TokenAccount>> {
        let account_data = match self.rpc_client.get_account_data(address) {
            Ok(data) => data,
            Err(solana_client::client_error::ClientError {
                kind:
                    solana_client::client_error::ClientErrorKind::RpcError(
                        solana_client::rpc_request::RpcError::RpcResponseError { .. },
                    ),
                ..
            }) => return Ok(None),
            Err(e) => return Err(ClientError::Rpc(e)),
        };

        let token_account: TokenAccount =
            AccountDeserialize::try_deserialize_unchecked(&mut &account_data[..]).map_err(|e| {
                ClientError::SplToken(format!("Failed to deserialize token account: {}", e))
            })?;

        Ok(Some(token_account))
    }

    /// Convenience method to check if a mint is WSOL
    pub fn is_wsol_mint(&self, mint: &Pubkey) -> bool {
        *mint == spl_token::native_mint::id()
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
        self.address_finder.program_id()
    }

    /// Get the RPC client (for advanced operations)
    pub fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }
}

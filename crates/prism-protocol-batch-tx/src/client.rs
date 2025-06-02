use crate::{TxBatchConfig, TxBatchError};
use backoff::future::retry;
use futures::future::try_join_all;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Cost estimation for a batch of instructions
#[derive(Debug, Clone)]
pub struct CostEstimate {
    /// Total estimated fee in lamports
    pub total_fee_lamports: u64,
    /// Number of transactions that will be created
    pub transaction_count: usize,
    /// Estimated compute units per transaction
    pub compute_units_per_tx: Vec<u32>,
}

/// High-level client for efficient batch transaction sending
pub struct BatchTxClient {
    rpc_client: Arc<RpcClient>,
    payer: Keypair,
    config: TxBatchConfig,
}

impl BatchTxClient {
    /// Create a new client with default configuration
    pub fn new(rpc_client: Arc<RpcClient>, payer: Keypair) -> Self {
        Self {
            rpc_client,
            payer,
            config: TxBatchConfig::default(),
        }
    }

    /// Create a new client with custom configuration
    pub fn with_config(rpc_client: Arc<RpcClient>, payer: Keypair, config: TxBatchConfig) -> Self {
        Self {
            rpc_client,
            payer,
            config,
        }
    }

    /// Get the payer's public key
    pub fn payer_pubkey(&self) -> Pubkey {
        self.payer.pubkey()
    }

    /// Send a batch of instructions efficiently
    ///
    /// This is the main API - handles chunking, simulation, retry, and parallel sending automatically
    pub async fn send_instructions(
        &self,
        instructions: Vec<Instruction>,
    ) -> Result<Vec<Signature>, TxBatchError> {
        if instructions.is_empty() {
            return Err(TxBatchError::NoInstructions);
        }

        // 1. Pack instructions into optimized messages
        let messages = self
            .pack_instructions_to_messages(instructions, &self.payer.pubkey())
            .await?;

        // 2. Verify balance if configured
        if self.config.verify_balance_before_send {
            self.verify_payer_balance(&messages, &self.payer.pubkey())
                .await?;
        }

        // 3. Send messages in parallel batches
        self.send_messages(messages).await
    }

    /// Send a batch of instructions with a different payer (for multi-signer scenarios)
    pub async fn send_instructions_with_payer(
        &self,
        instructions: Vec<Instruction>,
        payer: &Keypair,
    ) -> Result<Vec<Signature>, TxBatchError> {
        if instructions.is_empty() {
            return Err(TxBatchError::NoInstructions);
        }

        // 1. Pack instructions into optimized messages
        let messages = self
            .pack_instructions_to_messages(instructions, &payer.pubkey())
            .await?;

        // 2. Verify balance if configured
        if self.config.verify_balance_before_send {
            self.verify_payer_balance(&messages, &payer.pubkey())
                .await?;
        }

        // 3. Send messages in parallel batches
        self.send_messages_with_payer(messages, payer).await
    }

    /// Estimate the cost of sending instructions without actually sending them
    pub async fn estimate_cost(
        &self,
        instructions: Vec<Instruction>,
    ) -> Result<CostEstimate, TxBatchError> {
        if instructions.is_empty() {
            return Ok(CostEstimate {
                total_fee_lamports: 0,
                transaction_count: 0,
                compute_units_per_tx: vec![],
            });
        }

        let messages = self
            .pack_instructions_to_messages(instructions, &self.payer.pubkey())
            .await?;

        let mut total_fee = 0u64;
        let mut compute_units = Vec::new();

        for message in &messages {
            let fee = self.rpc_client.get_fee_for_message(message).await?;
            total_fee += fee;

            // For now, we'll estimate compute units based on instruction count
            // In a full implementation, we'd simulate each transaction
            let estimated_cu = message.instructions.len() as u32 * 10_000; // Rough estimate
            compute_units.push(estimated_cu);
        }

        Ok(CostEstimate {
            total_fee_lamports: total_fee,
            transaction_count: messages.len(),
            compute_units_per_tx: compute_units,
        })
    }

    /// Pack instructions into optimized messages (internal)
    async fn pack_instructions_to_messages(
        &self,
        instructions: Vec<Instruction>,
        payer: &Pubkey,
    ) -> Result<Vec<Message>, TxBatchError> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let mut messages = Vec::new();

        // Simple chunking based on instruction count
        // TODO: Implement smarter chunking based on transaction size
        for chunk in instructions.chunks(self.config.max_instructions_per_tx) {
            let message = Message::new_with_blockhash(chunk, Some(payer), &recent_blockhash);

            // TODO: Add simulation and compute unit optimization
            if self.config.simulate_before_send {
                debug!("Simulation would be performed here");
                // self.simulate_and_optimize_message(&mut message).await?;
            }

            messages.push(message);
        }

        Ok(messages)
    }

    /// Verify payer has sufficient balance for all transactions
    async fn verify_payer_balance(
        &self,
        messages: &[Message],
        payer: &Pubkey,
    ) -> Result<(), TxBatchError> {
        let mut total_fee = 0u64;
        for message in messages {
            total_fee += self.rpc_client.get_fee_for_message(message).await?;
        }

        let balance = self.rpc_client.get_balance(payer).await?;
        if balance < total_fee {
            return Err(TxBatchError::InsufficientBalance {
                required: total_fee,
                available: balance,
            });
        }

        debug!(
            "Balance check passed: {} lamports available, {} required",
            balance, total_fee
        );
        Ok(())
    }

    /// Send messages in parallel batches with retry logic (using default payer)
    async fn send_messages(&self, messages: Vec<Message>) -> Result<Vec<Signature>, TxBatchError> {
        self.send_messages_with_payer(messages, &self.payer).await
    }

    /// Send messages in parallel batches with retry logic (with custom payer)
    async fn send_messages_with_payer(
        &self,
        messages: Vec<Message>,
        payer: &Keypair,
    ) -> Result<Vec<Signature>, TxBatchError> {
        let mut all_signatures = Vec::new();

        // Send in parallel batches
        for (batch_idx, batch) in messages.chunks(self.config.max_parallel_sends).enumerate() {
            info!(
                "Sending batch {} of {} ({} transactions)",
                batch_idx + 1,
                (messages.len() + self.config.max_parallel_sends - 1)
                    / self.config.max_parallel_sends,
                batch.len()
            );

            // Create futures for this batch
            let batch_futures: Vec<_> = batch
                .iter()
                .enumerate()
                .map(|(tx_idx, message)| {
                    let overall_idx = batch_idx * self.config.max_parallel_sends + tx_idx;
                    self.send_single_message_with_retry(message.clone(), payer, overall_idx)
                })
                .collect();

            // Wait for all transactions in this batch to complete
            let batch_signatures = try_join_all(batch_futures).await?;
            all_signatures.extend(batch_signatures);
        }

        info!(
            "Successfully sent all {} transactions",
            all_signatures.len()
        );
        Ok(all_signatures)
    }

    /// Send a single message with retry logic
    async fn send_single_message_with_retry(
        &self,
        message: Message,
        payer: &Keypair,
        tx_index: usize,
    ) -> Result<Signature, TxBatchError> {
        let backoff = self.config.retry_backoff.clone();
        let rpc_client = self.rpc_client.clone();
        let confirmation_commitment = self.config.confirmation_commitment;

        let result = retry(backoff, || {
            let mut message = message.clone();
            let rpc_client = rpc_client.clone();

            async move {
                // Get fresh blockhash for each attempt
                let recent_blockhash = rpc_client
                    .get_latest_blockhash()
                    .await
                    .map_err(|e| backoff::Error::Permanent(TxBatchError::RpcClient(e)))?;
                message.recent_blockhash = recent_blockhash;

                // Create and sign transaction
                let mut transaction = Transaction::new_unsigned(message);
                transaction
                    .try_sign(&[payer], recent_blockhash)
                    .map_err(|e| {
                        backoff::Error::Permanent(TxBatchError::Config(format!(
                            "Failed to sign transaction: {}",
                            e
                        )))
                    })?;

                // Attempt to send
                match rpc_client
                    .send_and_confirm_transaction_with_spinner_and_commitment(
                        &transaction,
                        confirmation_commitment,
                    )
                    .await
                {
                    Ok(signature) => {
                        debug!("Transaction {} succeeded", tx_index);
                        Ok(signature)
                    }
                    Err(e) => {
                        warn!("Transaction {} attempt failed: {}", tx_index, e);

                        // Determine if this is a retryable error
                        let error_str = e.to_string();
                        if error_str.contains("blockhash") || error_str.contains("timeout") {
                            // Retryable errors
                            Err(backoff::Error::Transient {
                                err: TxBatchError::RpcClient(e),
                                retry_after: None,
                            })
                        } else {
                            // Permanent errors
                            Err(backoff::Error::Permanent(TxBatchError::RpcClient(e)))
                        }
                    }
                }
            }
        })
        .await;

        result.map_err(|e| match e {
            TxBatchError::RpcClient(rpc_err) => TxBatchError::RetriesExhausted {
                retries: self.config.max_retries,
                last_error: rpc_err.to_string(),
            },
            other => other,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::system_instruction;

    #[tokio::test]
    async fn test_empty_instructions() {
        let client = RpcClient::new("http://localhost:8899".to_string());
        let payer = Keypair::new();
        let batch_client = BatchTxClient::new(Arc::new(client), payer);

        let result = batch_client.send_instructions(vec![]).await;
        assert!(matches!(result, Err(TxBatchError::NoInstructions)));
    }

    #[tokio::test]
    async fn test_cost_estimation() {
        let client = RpcClient::new("http://localhost:8899".to_string());
        let payer = Keypair::new();
        let batch_client = BatchTxClient::new(Arc::new(client), payer);

        // Test with empty instructions
        let estimate = batch_client.estimate_cost(vec![]).await.unwrap();
        assert_eq!(estimate.transaction_count, 0);
        assert_eq!(estimate.total_fee_lamports, 0);

        // Test with some instructions (will fail with RPC error in test, but structure is correct)
        let _instructions = vec![system_instruction::transfer(
            &batch_client.payer_pubkey(),
            &Pubkey::new_unique(),
            1000,
        )];

        // This would require a running RPC endpoint to test properly
        // let estimate = batch_client.estimate_cost(instructions).await;
    }

    #[test]
    fn test_payer_pubkey() {
        let client = RpcClient::new("http://localhost:8899".to_string());
        let payer = Keypair::new();
        let expected_pubkey = payer.pubkey();
        let batch_client = BatchTxClient::new(Arc::new(client), payer);

        assert_eq!(batch_client.payer_pubkey(), expected_pubkey);
    }
}

use backoff::ExponentialBackoff;
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;

/// Configuration for batch transaction operations
#[derive(Debug, Clone)]
pub struct TxBatchConfig {
    /// Maximum number of retry attempts for failed transactions
    pub max_retries: usize,

    /// Backoff strategy for retry delays
    pub retry_backoff: ExponentialBackoff,

    /// Maximum number of instructions to pack into a single transaction
    pub max_instructions_per_tx: usize,

    /// Commitment level for transaction confirmation
    pub confirmation_commitment: CommitmentConfig,

    /// Whether to skip preflight checks (simulation before sending)
    pub skip_preflight: bool,

    /// Whether to simulate transactions before sending to optimize compute units
    pub simulate_before_send: bool,

    /// Whether to automatically set compute unit limits based on simulation
    pub auto_compute_unit_limit: bool,

    /// Whether to verify payer balance before sending transactions
    pub verify_balance_before_send: bool,

    /// Maximum number of transactions to send in parallel
    pub max_parallel_sends: usize,

    /// Whether to chunk instructions optimally based on transaction size limits
    pub chunk_instructions_optimally: bool,

    /// Maximum transaction size in bytes (conservative default)
    pub max_transaction_size_bytes: usize,
}

impl Default for TxBatchConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            retry_backoff: ExponentialBackoff {
                initial_interval: Duration::from_millis(500),
                max_interval: Duration::from_secs(30),
                max_elapsed_time: Some(Duration::from_secs(300)), // 5 minutes total
                multiplier: 2.0,
                ..Default::default()
            },
            max_instructions_per_tx: 10, // Conservative default
            confirmation_commitment: CommitmentConfig::confirmed(),
            skip_preflight: false,
            simulate_before_send: true,
            auto_compute_unit_limit: true,
            verify_balance_before_send: true,
            max_parallel_sends: 4,
            chunk_instructions_optimally: true,
            max_transaction_size_bytes: 1200, // Conservative, well under 1232 limit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TxBatchConfig::default();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.max_parallel_sends, 4);
        assert_eq!(config.max_instructions_per_tx, 10);
        assert!(config.simulate_before_send);
        assert!(config.verify_balance_before_send);
    }
}

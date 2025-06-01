use thiserror::Error;

/// Errors that can occur during batch transaction operations
#[derive(Error, Debug)]
pub enum TxBatchError {
    #[error("RPC client error: {0}")]
    RpcClient(#[from] solana_client::client_error::ClientError),
    
    #[error("Transaction failed after {retries} retries: {last_error}")]
    RetriesExhausted { retries: usize, last_error: String },
    
    #[error("Transaction packing failed: {0}")]
    Packing(String),
    
    #[error("Blockhash expired during transaction processing")]
    BlockhashExpired,
    
    #[error("Insufficient balance: need {required} lamports, have {available}")]
    InsufficientBalance { required: u64, available: u64 },
    
    #[error("Transaction simulation failed: {0}")]
    SimulationFailed(String),
    
    #[error("Transaction too large: {size} bytes (max: {max})")]
    TransactionTooLarge { size: usize, max: usize },
    
    #[error("No instructions provided")]
    NoInstructions,
    
    #[error("Configuration error: {0}")]
    Config(String),
} 
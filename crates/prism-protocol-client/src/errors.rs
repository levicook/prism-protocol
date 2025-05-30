use thiserror::Error;

pub type ClientResult<T> = Result<T, ClientError>;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("RPC error: {0}")]
    Rpc(#[from] solana_client::client_error::ClientError),

    #[error("Program error: {0}")]
    Program(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Invalid account data: {0}")]
    InvalidAccountData(String),

    #[error("SPL Token error: {0}")]
    SplToken(String),

    #[error("Transaction simulation failed: {0}")]
    SimulationFailed(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

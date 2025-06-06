use thiserror::Error;

pub type DbResult<T> = Result<T, DbError>;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid pubkey: {0}")]
    InvalidPubkey(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Connection error: {0}")]
    Connection(String),
}

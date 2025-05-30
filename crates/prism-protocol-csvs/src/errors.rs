use thiserror::Error;

pub type CsvResult<T> = Result<T, CsvError>;

#[derive(Error, Debug)]
pub enum CsvError {
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Schema validation error: {0}")]
    SchemaValidation(String),

    #[error("Invalid pubkey: {0}")]
    InvalidPubkey(String),

    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },

    #[error("Inconsistent data between CSV files: {0}")]
    DataInconsistency(String),

    #[error("Missing required header: {0}")]
    MissingHeader(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

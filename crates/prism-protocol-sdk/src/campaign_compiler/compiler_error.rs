use rust_decimal::Decimal;

#[derive(Debug, thiserror::Error)]
pub enum CompilerError {
    #[error(transparent)]
    Allocation(#[from] crate::budget_allocation::AllocationError),

    #[error(transparent)] // TODO -- i think this can be removed
    ClaimTree(#[from] anchor_lang::prelude::Error), // THIS IS A HACK (merkle create didn't encapsulate errors well)

    #[error("Campaign not found")]
    CampaignNotFound,

    #[error(transparent)]
    CampaignDatabase(#[from] crate::campaign_database::Error),

    #[error(transparent)]
    Csv(#[from] prism_protocol_csvs::CsvError),

    #[error(transparent)]
    Decimal(#[from] rust_decimal::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Invalid claim tree type: {0}")]
    InvalidClaimTreeType(String),

    #[error("Merkle root is required")]
    MerkleRootIsRequired,

    #[error(transparent)]
    ParsePubkey(#[from] solana_sdk::pubkey::ParsePubkeyError),

    #[error(transparent)]
    SeaOrm(#[from] sea_orm::DbErr),

    #[error(transparent)]
    TryFromInt(#[from] std::num::TryFromIntError),

    #[error(
        "Vault limit exceeded: {claimant_count} claimants ÷ {claimants_per_vault} per vault requires {vault_count} vaults (max 255)"
    )]
    VaultLimitExceeded {
        claimant_count: Decimal,
        claimants_per_vault: Decimal,
        vault_count: Decimal,
    },
}

pub type CompilerResult<T> = std::result::Result<T, CompilerError>;

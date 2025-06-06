mod campaign_csv_writer;
mod campaign_writer;
mod claim_tree;
mod cohort_writer;
mod cohorts_csv_writer;

use prism_protocol_csvs::{CampaignCsvRow, CohortsCsvRow, validate_csv_consistency};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer as _;

use crate::AddressFinder;
use crate::campaign_compiler::campaign_csv_writer::import_campaign_csv_rows;
use crate::campaign_compiler::campaign_writer::import_campaign;
use crate::campaign_compiler::cohort_writer::import_cohorts;
use crate::campaign_compiler::cohorts_csv_writer::import_cohorts_csv_rows;
use crate::campaign_database::new_writeable_campaign_db;

pub use claim_tree::{ClaimTree, ClaimTreeType};

pub async fn compile_campaign(
    campaign_admin: Pubkey,   // campaign admin
    campaign_budget: Decimal, // total budget in human-readable tokens
    campaign_mint: Pubkey,    // SPL token mint for the campaign
    mint_decimals: u8, // number of decimals for the token mint (critical for budget allocation)
    campaign_csv_rows: &[CampaignCsvRow],
    cohorts_csv_rows: &[CohortsCsvRow],
    claimants_per_vault: usize, // ratio that determines rent -vs- claim contention
    claim_tree_type: ClaimTreeType,
) -> CompilerResult<(Keypair, DatabaseConnection)> {
    validate_csv_consistency(campaign_csv_rows, cohorts_csv_rows)?; // fail fast if the csvs are invalid

    let campaign_keypair = Keypair::new();
    let campaign_address = campaign_keypair.pubkey();
    let address_finder = AddressFinder::new(campaign_admin, campaign_address, campaign_mint);

    let db = new_writeable_campaign_db().await?;

    import_campaign_csv_rows(&db, campaign_csv_rows).await?;
    import_cohorts_csv_rows(&db, cohorts_csv_rows).await?;

    import_campaign(
        &db,
        campaign_address,
        campaign_admin,
        campaign_budget,
        campaign_mint,
        mint_decimals,
        claimants_per_vault,
        claim_tree_type,
    )
    .await?;

    import_cohorts(&address_finder, &db).await?;

    Ok((campaign_keypair, db))
}

#[derive(Debug, thiserror::Error)]
pub enum CompilerError {
    #[error(transparent)]
    Allocation(#[from] crate::budget_allocation::AllocationError),

    #[error(transparent)]
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

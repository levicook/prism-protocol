mod campaign_csv_writer;
mod campaign_writer;
mod claim_tree;
mod cohort_writer;
mod cohorts_csv_writer;
mod compiled_campaign_database;
mod compiler_error;

use {
    crate::{new_writeable_campaign_db, AddressFinder},
    prism_protocol_csvs::{validate_csv_consistency, CampaignCsvRow, CohortsCsvRow},
    rust_decimal::Decimal,
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer as _},
};

pub use {
    claim_tree::{ClaimTree, ClaimTreeType},
    compiled_campaign_database::*,
    compiler_error::{CompilerError, CompilerResult},
};

pub async fn compile_campaign(
    campaign_admin: Pubkey,   // campaign admin
    campaign_budget: Decimal, // total budget in human-readable tokens
    campaign_mint: Pubkey,    // SPL token mint for the campaign
    mint_decimals: u8, // number of decimals for the token mint (critical for budget allocation)
    campaign_csv_rows: &[CampaignCsvRow],
    cohorts_csv_rows: &[CohortsCsvRow],
    claimants_per_vault: usize, // ratio that determines rent -vs- claim contention
    claim_tree_type: ClaimTreeType,
) -> CompilerResult<CompiledCampaignDatabase> {
    validate_csv_consistency(campaign_csv_rows, cohorts_csv_rows)?; // fail fast if the csvs are invalid

    let campaign_keypair = Keypair::new();
    let campaign_address = campaign_keypair.pubkey();
    let address_finder = AddressFinder::new(campaign_admin, campaign_address, campaign_mint);

    let db = new_writeable_campaign_db().await?;

    campaign_csv_writer::import_campaign_csv_rows(&db, campaign_csv_rows).await?;
    cohorts_csv_writer::import_cohorts_csv_rows(&db, cohorts_csv_rows).await?;

    campaign_writer::import_campaign(
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

    cohort_writer::import_cohorts(&address_finder, &db).await?;

    Ok(CompiledCampaignDatabase::new_with_keypair(
        address_finder,
        db,
        campaign_keypair,
    ))
}

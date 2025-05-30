/*!
# Prism Protocol CSV Schema Definitions

This crate provides the **authoritative CSV schemas** used throughout Prism Protocol.

## Purpose

This crate serves as the **single source of truth** for CSV data contracts between:

- **`generate-fixtures`** (producer) → Creates CSV files
- **`compile-campaign`** (consumer) → Reads CSV files
- **API Server** (future consumer) → Will accept CSV uploads
- **CLI Commands** (consumers) → Process campaign data

## Schema Files

### Campaign CSV (`campaign.csv`)
Contains claimant information with columns:
- `cohort`: Cohort identifier
- `claimant`: Solana public key (base58)
- `entitlements`: Number of entitlements (u64)

### Cohorts CSV (`cohorts.csv`)
Contains cohort configuration with columns:
- `cohort`: Cohort identifier
- `amount_per_entitlement`: Amount of tokens per entitlement for this cohort (u64)

## Versioning

All CSV schemas include version metadata to handle evolution:
- Current version: `1.0`
- Version header: `# prism-protocol-csv-version: 1.0`

## Usage

```rust
use prism_protocol_csvs::{CampaignRow, CohortsRow, read_campaign_csv, read_cohorts_csv, validate_csv_consistency, CsvResult};

fn example() -> CsvResult<()> {
    // Read and validate CSV files
    let campaign_rows = read_campaign_csv("campaign.csv")?;
    let cohorts_rows = read_cohorts_csv("cohorts.csv")?;

    // Validate consistency between files
    validate_csv_consistency(&campaign_rows, &cohorts_rows)?;

    Ok(())
}
```
*/

pub mod errors;
pub mod schemas;
pub mod validation;

// Re-export main types for convenience
pub use errors::{CsvError, CsvResult};
pub use schemas::{CampaignRow, CohortsRow, CURRENT_SCHEMA_VERSION};
pub use validation::{
    read_campaign_csv, read_cohorts_csv, validate_csv_consistency, write_campaign_csv,
    write_cohorts_csv,
};

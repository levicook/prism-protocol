/*!
# CSV Validation & I/O

This module provides validation functions for CSV files that ensure consistency
between `generate-fixtures` and `compile-campaign` operations.
*/

use crate::{
    errors::{CsvError, CsvResult},
    schemas::{CampaignCsvRow, CohortsCsvRow, CAMPAIGN_CSV_HEADERS, COHORTS_CSV_HEADERS},
};
use csv::{Reader, Writer};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

// ================================================================================================
// CSV Reading with Validation
// ================================================================================================

/// Read and validate a campaign CSV file
pub fn read_campaign_csv<P: AsRef<Path>>(path: P) -> CsvResult<Vec<CampaignCsvRow>> {
    let file = File::open(path)?;
    let mut rdr = Reader::from_reader(file);

    // Validate headers
    let headers = rdr.headers()?;
    validate_headers(headers.iter(), CAMPAIGN_CSV_HEADERS, "campaign.csv")?;

    // Read and deserialize rows
    let mut rows = Vec::new();
    for result in rdr.deserialize() {
        let row: CampaignCsvRow = result?;
        rows.push(row);
    }

    if rows.is_empty() {
        return Err(CsvError::SchemaValidation(
            "Campaign CSV file is empty".to_string(),
        ));
    }

    Ok(rows)
}

/// Read and validate a cohorts CSV file
pub fn read_cohorts_csv<P: AsRef<Path>>(path: P) -> CsvResult<Vec<CohortsCsvRow>> {
    let file = File::open(path)?;
    let mut rdr = Reader::from_reader(file);

    // Validate headers
    let headers = rdr.headers()?;
    validate_headers(headers.iter(), COHORTS_CSV_HEADERS, "cohorts.csv")?;

    // Read and deserialize rows
    let mut rows = Vec::new();
    for result in rdr.deserialize() {
        let row: CohortsCsvRow = result?;
        rows.push(row);
    }

    if rows.is_empty() {
        return Err(CsvError::SchemaValidation(
            "Cohorts CSV file is empty".to_string(),
        ));
    }

    Ok(rows)
}

// ================================================================================================
// CSV Writing
// ================================================================================================

/// Write campaign CSV with proper headers and validation
pub fn write_campaign_csv<P: AsRef<Path>>(path: P, rows: &[CampaignCsvRow]) -> CsvResult<()> {
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    // Write data rows (csv crate automatically writes headers)
    for row in rows {
        wtr.serialize(row)?;
    }

    wtr.flush()?;
    Ok(())
}

/// Write cohorts CSV with proper headers and validation
pub fn write_cohorts_csv<P: AsRef<Path>>(path: P, rows: &[CohortsCsvRow]) -> CsvResult<()> {
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    // Write data rows (csv crate automatically writes headers)
    for row in rows {
        wtr.serialize(row)?;
    }

    wtr.flush()?;
    Ok(())
}

// ================================================================================================
// Cross-CSV Validation
// ================================================================================================

/// Validate consistency between campaign and cohorts CSV files
///
/// Ensures:
/// - All cohorts referenced in campaign.csv exist in cohorts.csv
/// - No orphaned cohorts (cohorts defined but not used)
/// - Share percentages are valid (0.0-100.0) and sum to 100.0
pub fn validate_csv_consistency(
    campaign_rows: &[CampaignCsvRow],
    cohorts_rows: &[CohortsCsvRow],
) -> CsvResult<()> {
    // Build maps for efficient lookups
    let cohorts_map: HashMap<String, &CohortsCsvRow> = cohorts_rows
        .iter()
        .map(|row| (row.cohort.clone(), row))
        .collect();

    let mut campaign_cohorts = HashMap::new();

    // Collect campaign cohorts
    for row in campaign_rows {
        campaign_cohorts.insert(row.cohort.clone(), ());
    }

    // Check 1: All campaign cohorts must exist in cohorts.csv
    for cohort in campaign_cohorts.keys() {
        if !cohorts_map.contains_key(cohort) {
            return Err(CsvError::DataInconsistency(format!(
                "Cohort '{}' referenced in campaign.csv but not defined in cohorts.csv",
                cohort
            )));
        }
    }

    // Check 2: No orphaned cohorts in config
    for cohort in cohorts_map.keys() {
        if !campaign_cohorts.contains_key(cohort) {
            return Err(CsvError::DataInconsistency(format!(
                "Cohort '{}' defined in cohorts.csv but has no claimants in campaign.csv",
                cohort
            )));
        }
    }

    // Check 3: Validate share percentages
    validate_share_percentages(cohorts_rows)?;

    Ok(())
}

/// Validate that share percentages are valid and sum to 100%
pub fn validate_share_percentages(cohorts_rows: &[CohortsCsvRow]) -> CsvResult<()> {
    let mut total_percentage = Decimal::ZERO;
    let hundred = Decimal::from(100);

    for row in cohorts_rows {
        // Check individual percentage range
        if row.share_percentage < Decimal::ZERO || row.share_percentage > hundred {
            return Err(CsvError::InvalidFormat(format!(
                "Cohort '{}' has invalid share percentage: {}%. Must be between 0% and 100%",
                row.cohort, row.share_percentage
            )));
        }

        total_percentage += row.share_percentage;
    }

    // Check that total equals exactly 100%
    if total_percentage != hundred {
        return Err(CsvError::InvalidFormat(format!(
            "Share percentages must sum to exactly 100%, but got {}%",
            total_percentage
        )));
    }

    Ok(())
}

// ================================================================================================
// Header Validation
// ================================================================================================

fn validate_headers<'a, I>(actual: I, expected: &[&str], file_type: &str) -> CsvResult<()>
where
    I: Iterator<Item = &'a str>,
{
    let actual_headers: Vec<&str> = actual.collect();

    if actual_headers.len() != expected.len() {
        return Err(CsvError::SchemaValidation(format!(
            "{}: expected {} headers, found {}",
            file_type,
            expected.len(),
            actual_headers.len()
        )));
    }

    for (i, (actual, expected)) in actual_headers.iter().zip(expected.iter()).enumerate() {
        if actual != expected {
            return Err(CsvError::SchemaValidation(format!(
                "{}: header {} should be '{}', found '{}'",
                file_type,
                i + 1,
                expected,
                actual
            )));
        }
    }

    Ok(())
}

// ================================================================================================
// Tests
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_and_read_campaign_csv() {
        let rows = vec![
            CampaignCsvRow {
                cohort: "earlyAdopters".to_string(),
                claimant: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
                entitlements: 100,
            },
            CampaignCsvRow {
                cohort: "powerUsers".to_string(),
                claimant: Pubkey::from_str("11111111111111111111111111111113").unwrap(),
                entitlements: 200,
            },
        ];

        let temp_file = NamedTempFile::new().unwrap();
        write_campaign_csv(temp_file.path(), &rows).unwrap();
        let read_rows = read_campaign_csv(temp_file.path()).unwrap();

        assert_eq!(rows, read_rows);
    }

    #[test]
    fn test_write_and_read_cohorts_csv() {
        let rows = vec![
            CohortsCsvRow {
                cohort: "earlyAdopters".to_string(),
                share_percentage: Decimal::from(60),
            },
            CohortsCsvRow {
                cohort: "powerUsers".to_string(),
                share_percentage: Decimal::from(40),
            },
        ];

        let temp_file = NamedTempFile::new().unwrap();
        write_cohorts_csv(temp_file.path(), &rows).unwrap();
        let read_rows = read_cohorts_csv(temp_file.path()).unwrap();

        assert_eq!(rows, read_rows);
    }

    #[test]
    fn test_csv_consistency_validation() {
        let campaign_rows = vec![
            CampaignCsvRow {
                cohort: "earlyAdopters".to_string(),
                claimant: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
                entitlements: 50,
            },
            CampaignCsvRow {
                cohort: "earlyAdopters".to_string(),
                claimant: Pubkey::from_str("11111111111111111111111111111113").unwrap(),
                entitlements: 50,
            },
        ];

        let cohort_config_rows = vec![CohortsCsvRow {
            cohort: "earlyAdopters".to_string(),
            share_percentage: Decimal::from(100),
        }];

        // Should pass validation
        validate_csv_consistency(&campaign_rows, &cohort_config_rows).unwrap();

        // Should fail with orphaned cohort in config
        let bad_config_rows = vec![
            CohortsCsvRow {
                cohort: "earlyAdopters".to_string(),
                share_percentage: Decimal::from(60),
            },
            CohortsCsvRow {
                cohort: "orphanedCohort".to_string(),
                share_percentage: Decimal::from(40),
            },
        ];

        let result = validate_csv_consistency(&campaign_rows, &bad_config_rows);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("has no claimants in campaign.csv"));
    }

    #[test]
    fn test_share_percentage_validation() {
        let valid_rows = vec![
            CohortsCsvRow {
                cohort: "early".to_string(),
                share_percentage: Decimal::from(70),
            },
            CohortsCsvRow {
                cohort: "power".to_string(),
                share_percentage: Decimal::from(30),
            },
        ];

        // Should pass - sums to 100%
        validate_share_percentages(&valid_rows).unwrap();

        // Test precise decimal values
        let precise_rows = vec![
            CohortsCsvRow {
                cohort: "early".to_string(),
                share_percentage: Decimal::from_str("60.5").unwrap(),
            },
            CohortsCsvRow {
                cohort: "power".to_string(),
                share_percentage: Decimal::from_str("39.5").unwrap(),
            },
        ];

        // Should pass - precise decimals sum to exactly 100%
        validate_share_percentages(&precise_rows).unwrap();

        // Should fail - doesn't sum to 100%
        let invalid_sum_rows = vec![
            CohortsCsvRow {
                cohort: "early".to_string(),
                share_percentage: Decimal::from(70),
            },
            CohortsCsvRow {
                cohort: "power".to_string(),
                share_percentage: Decimal::from(40), // Total = 110%
            },
        ];

        let result = validate_share_percentages(&invalid_sum_rows);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must sum to exactly 100%"));

        // Should fail - invalid range
        let invalid_range_rows = vec![CohortsCsvRow {
            cohort: "invalid".to_string(),
            share_percentage: Decimal::from(150), // > 100%
        }];

        let result = validate_share_percentages(&invalid_range_rows);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Must be between 0% and 100%"));
    }
}

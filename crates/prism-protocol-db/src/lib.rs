/*!
# Prism Protocol Database Management

This crate provides unified database access for Prism Protocol campaigns.

## Purpose

This crate eliminates the technical debt of scattered `Connection::open()` calls
throughout the CLI commands by providing a clean, unified interface.

## Features

- **Unified Connection Management**: Single database connection interface
- **Campaign Operations**: Read campaign, cohort, and eligibility information
- **Transaction Management**: Proper transaction handling for updates
- **Error Handling**: Consistent error types across all database operations
- **Type Safety**: Proper Pubkey parsing and hex validation

## Usage

```rust
use prism_protocol_db::{CampaignDatabase, DbResult};
use std::path::Path;

fn example() -> DbResult<()> {
    let db = CampaignDatabase::open(Path::new("campaign.db"))?;

    let campaign_info = db.read_campaign_info()?;
    println!("Campaign mint: {}", campaign_info.mint);

    let cohorts = db.read_cohorts()?;
    println!("Found {} cohorts", cohorts.len());

    Ok(())
}
```

## Implementation Status

✅ **COMPLETED** - Core database interface with essential operations for API server
*/

pub mod database;
pub mod errors;
pub mod schema;

// Re-export main types for convenience
pub use database::{
    CampaignDatabase, CampaignInfo, ClaimProof, CohortInfo, EligibilityInfo, VaultRequirement,
};
pub use errors::{DbError, DbResult};
pub use schema::{check_schema, get_schema_version, initialize_database, SCHEMA_VERSION};

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;
    use tempfile::NamedTempFile;

    /// Test database creation and schema validation
    #[test]
    fn test_database_creation_and_schema() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        // Create database with schema
        let db = CampaignDatabase::create(db_path).unwrap();

        // Verify schema was created properly
        assert!(db.verify_schema().unwrap());

        // Should return an error for empty database (no campaign data)
        let result = db.read_campaign_info();
        assert!(result.is_err());

        if let Err(DbError::InvalidConfig(_)) = result {
            // Expected: no campaign data found
        } else {
            panic!("Expected InvalidConfig error for empty campaign data");
        }
    }

    /// Test schema version management
    #[test]
    fn test_schema_version() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        // Create database with schema
        let db = CampaignDatabase::create(db_path).unwrap();

        // Check schema version
        let version = get_schema_version(db.connection()).unwrap();
        assert_eq!(version, Some(SCHEMA_VERSION));
    }

    /// Test opening existing database without schema
    #[test]
    fn test_open_empty_database() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        // Open empty database (no schema)
        let db = CampaignDatabase::open(db_path).unwrap();

        // Schema should not exist
        assert!(!db.verify_schema().unwrap());

        // Should return database error for missing table
        let result = db.read_campaign_info();
        assert!(result.is_err());

        if let Err(DbError::Database(_)) = result {
            // Expected: rusqlite error for missing table
        } else {
            panic!("Expected database error for missing table");
        }
    }

    /// Test error handling for invalid paths
    #[test]
    fn test_invalid_database_path() {
        let result = CampaignDatabase::open(std::path::Path::new("/nonexistent/path/db.sqlite"));
        assert!(result.is_err());

        if let Err(DbError::Connection(_)) = result {
            // Expected: connection error
        } else {
            panic!("Expected connection error for invalid path");
        }
    }

    /// Test claimant eligibility with properly initialized database
    #[test]
    fn test_empty_eligibility_query_with_schema() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        // Create database with proper schema
        let db = CampaignDatabase::create(db_path).unwrap();
        let test_pubkey = Pubkey::from_str("11111111111111111111111111111112").unwrap();

        // Should return empty results, not error
        let result = db.read_claimant_eligibility(&test_pubkey);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}

/*!
# CSV Schema Definitions

This module defines the authoritative CSV schemas used throughout Prism Protocol.
These schemas serve as the contract between:
- `generate-fixtures` (producer)
- `compile-campaign` (consumer)
- Future API endpoints (consumer)

## Schema Versioning

Each schema includes version information to handle evolution over time.
*/

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Current schema version for all CSV formats
pub const CURRENT_SCHEMA_VERSION: &str = "1.0";

/// Schema version header that should appear in CSV metadata
pub const VERSION_HEADER: &str = "# prism-protocol-csv-version";

// ================================================================================================
// Campaign CSV Schema
// ================================================================================================

/// Expected headers for campaign.csv in exact order
pub const CAMPAIGN_CSV_HEADERS: &[&str] = &["cohort", "claimant", "entitlements"];

/// Row structure for campaign.csv
///
/// **File**: `campaign.csv`
/// **Purpose**: Contains claimant eligibility data
/// **Producer**: `generate-fixtures` command
/// **Consumers**: `compile-campaign` command, future API endpoints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CampaignRow {
    /// Cohort identifier (e.g., "earlyAdopters", "powerUsers")
    pub cohort: String,

    /// Claimant's Solana public key in base58 format
    #[serde(
        deserialize_with = "deserialize_pubkey",
        serialize_with = "serialize_pubkey"
    )]
    pub claimant: Pubkey,

    /// Number of entitlements this claimant can claim
    pub entitlements: u64,
}

// ================================================================================================
// Cohorts CSV Schema
// ================================================================================================

/// Expected headers for cohorts.csv in exact order
pub const COHORTS_CSV_HEADERS: &[&str] = &["cohort", "merkle_root", "total_entitlements"];

/// Row structure for cohorts.csv
///
/// **File**: `cohorts.csv`
/// **Purpose**: Contains cohort configuration and merkle tree data
/// **Producer**: `compile-campaign` command
/// **Consumers**: `deploy-campaign` command, future API endpoints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CohortsRow {
    /// Cohort identifier - must match cohorts referenced in campaign.csv
    pub cohort: String,

    /// Merkle root hash for this cohort in hex format (without 0x prefix)
    #[serde(deserialize_with = "deserialize_hex", serialize_with = "serialize_hex")]
    pub merkle_root: [u8; 32],

    /// Total number of entitlements across all claimants in this cohort
    pub total_entitlements: u64,
}

// ================================================================================================
// Custom Serde Functions
// ================================================================================================

/// Deserialize base58 string to Pubkey
fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

/// Serialize Pubkey to base58 string
fn serialize_pubkey<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&pubkey.to_string())
}

/// Deserialize hex string to [u8; 32]
fn deserialize_hex<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
    if bytes.len() != 32 {
        return Err(serde::de::Error::custom(format!(
            "Expected 32 bytes, got {}",
            bytes.len()
        )));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

/// Serialize [u8; 32] to hex string
fn serialize_hex<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(bytes))
}

// ================================================================================================
// Tests
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_campaign_csv_row_serialization() {
        let row = CampaignRow {
            cohort: "earlyAdopters".to_string(),
            claimant: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
            entitlements: 100,
        };

        // Test CSV serialization/deserialization
        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.serialize(&row).unwrap();
        let csv_data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();

        let mut rdr = csv::Reader::from_reader(csv_data.as_bytes());
        let deserialized: CampaignRow = rdr.deserialize().next().unwrap().unwrap();

        assert_eq!(row, deserialized);
    }

    #[test]
    fn test_cohorts_csv_row_serialization() {
        let merkle_root = [1u8; 32];
        let row = CohortsRow {
            cohort: "earlyAdopters".to_string(),
            merkle_root,
            total_entitlements: 1000,
        };

        // Test CSV serialization/deserialization
        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.serialize(&row).unwrap();
        let csv_data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();

        let mut rdr = csv::Reader::from_reader(csv_data.as_bytes());
        let deserialized: CohortsRow = rdr.deserialize().next().unwrap().unwrap();

        assert_eq!(row, deserialized);
    }

    #[test]
    fn test_hex_serialization_roundtrip() {
        let original = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
            0x89, 0xab, 0xcd, 0xef,
        ];
        let hex_str = hex::encode(original);
        let decoded = hex::decode(&hex_str).unwrap();
        let mut array = [0u8; 32];
        array.copy_from_slice(&decoded);
        assert_eq!(original, array);
    }
}

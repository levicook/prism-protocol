/*!
# Database Schema Management

This module contains the complete database schema for Prism Protocol campaigns
and provides migration/initialization functionality.
*/

use crate::{DbError, DbResult};
use rusqlite::Connection;

/// Current database schema version
pub const SCHEMA_VERSION: i32 = 1;

/// Initialize database with complete schema
pub fn initialize_database(conn: &Connection) -> DbResult<()> {
    // Create the complete schema in one transaction
    conn.execute_batch(
        r#"
        -- Campaign metadata and deployment tracking
        CREATE TABLE campaign (
            fingerprint TEXT PRIMARY KEY,
            mint TEXT NOT NULL,
            admin TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            deployed_at INTEGER,
            deployed_signature TEXT, -- transaction signature for campaign deployment
            activated_at INTEGER, -- timestamp when campaign was activated
            activation_signature TEXT -- transaction signature for campaign activation
        );

        -- Cohort information with merkle trees and deployment tracking
        CREATE TABLE cohorts (
            cohort_name TEXT PRIMARY KEY,
            merkle_root TEXT NOT NULL, -- hex-encoded [u8; 32]
            amount_per_entitlement INTEGER NOT NULL,
            vault_count INTEGER NOT NULL,
            claimant_count INTEGER NOT NULL,
            total_tokens_required INTEGER NOT NULL,
            deployed_at INTEGER,
            deployed_signature TEXT -- transaction signature for cohort deployment
        );

        -- Individual claimant eligibility and claim tracking
        CREATE TABLE claimants (
            claimant TEXT NOT NULL,
            cohort_name TEXT NOT NULL,
            entitlements INTEGER NOT NULL,
            assigned_vault_index INTEGER NOT NULL, -- index into vaults table
            assigned_vault_pubkey TEXT NOT NULL, -- hex-encoded pubkey for convenience
            merkle_proof TEXT NOT NULL, -- hex-encoded proof (comma-separated hashes)
            claimed_at INTEGER,
            claimed_signature TEXT, -- transaction signature for claim
            PRIMARY KEY (claimant, cohort_name),
            FOREIGN KEY (cohort_name) REFERENCES cohorts(cohort_name)
        );

        -- Vault information and funding tracking
        CREATE TABLE vaults (
            cohort_name TEXT NOT NULL,
            vault_index INTEGER NOT NULL,
            vault_pubkey TEXT NOT NULL, -- hex-encoded pubkey
            vault_keypair_path TEXT, -- optional: path to keypair file if generated
            required_tokens INTEGER NOT NULL,
            assigned_claimants INTEGER NOT NULL,
            created_at INTEGER, -- timestamp when vault PDA was created on-chain
            created_by_tx TEXT, -- transaction signature for vault creation
            funded_at INTEGER, -- timestamp when vault was funded with tokens
            funded_by_tx TEXT, -- transaction signature for vault funding
            funded_amount INTEGER, -- actual amount funded (for verification)
            funded_signature TEXT, -- newer field name for consistency
            PRIMARY KEY (cohort_name, vault_index),
            FOREIGN KEY (cohort_name) REFERENCES cohorts(cohort_name)
        );

        -- Indexes for efficient lookups
        CREATE INDEX idx_claimants_lookup ON claimants(claimant, cohort_name);
        CREATE INDEX idx_vaults_lookup ON vaults(cohort_name, vault_index);
        CREATE INDEX idx_claimants_cohort ON claimants(cohort_name);
        CREATE INDEX idx_vaults_cohort ON vaults(cohort_name);

        -- Schema version tracking
        CREATE TABLE schema_version (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );

        INSERT INTO schema_version (version, applied_at) 
        VALUES (?, datetime('now'));
        "#,
    )
    .map_err(|e| DbError::Database(e))?;

    // Insert current schema version
    conn.execute(
        "UPDATE schema_version SET applied_at = datetime('now') WHERE version = ?",
        [SCHEMA_VERSION],
    )
    .map_err(|e| DbError::Database(e))?;

    Ok(())
}

/// Check if database is properly initialized
pub fn check_schema(conn: &Connection) -> DbResult<bool> {
    // Check if campaign table exists
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='campaign'")
        .map_err(|e| DbError::Database(e))?;

    let mut rows = stmt
        .query_map([], |_row| Ok(()))
        .map_err(|e| DbError::Database(e))?;

    Ok(rows.next().is_some())
}

/// Get current schema version from database
pub fn get_schema_version(conn: &Connection) -> DbResult<Option<i32>> {
    // Check if schema_version table exists first
    let table_exists = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'")
        .and_then(|mut stmt| {
            let mut rows = stmt.query_map([], |_| Ok(()))?;
            Ok(rows.next().is_some())
        })
        .unwrap_or(false);

    if !table_exists {
        return Ok(None);
    }

    let mut stmt = conn
        .prepare("SELECT version FROM schema_version ORDER BY version DESC LIMIT 1")
        .map_err(|e| DbError::Database(e))?;

    let mut rows = stmt
        .query_map([], |row| {
            let version: i32 = row.get(0)?;
            Ok(version)
        })
        .map_err(|e| DbError::Database(e))?;

    if let Some(row) = rows.next() {
        Ok(Some(row.map_err(|e| DbError::Database(e))?))
    } else {
        Ok(None)
    }
}

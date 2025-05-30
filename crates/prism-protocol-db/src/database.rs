/*!
# Database Operations

Unified database interface for all campaign operations, replacing scattered
`Connection::open()` calls throughout the CLI commands.
*/

use crate::{
    schema::{check_schema, initialize_database},
    DbError, DbResult,
};
use rusqlite::{params, Connection};
use solana_sdk::pubkey::Pubkey;
use std::path::Path;
use std::str::FromStr;

/// Essential campaign information from database
#[derive(Debug, Clone)]
pub struct CampaignInfo {
    pub fingerprint: [u8; 32],
    pub mint: Pubkey,
    pub admin: Pubkey,
}

/// Cohort information with merkle data
#[derive(Debug, Clone)]
pub struct CohortInfo {
    pub name: String,
    pub merkle_root: [u8; 32],
    pub amount_per_entitlement: u64,
    pub vaults: Vec<Pubkey>,
    pub vault_count: usize,
}

/// Claimant eligibility information combining database and calculated data
#[derive(Debug, Clone)]
pub struct EligibilityInfo {
    pub cohort_name: String,
    pub cohort_merkle_root: [u8; 32],
    pub entitlements: u64,
    pub amount_per_entitlement: u64,
    pub total_tokens: u64,
    // Database state
    pub db_claimed: bool,
    pub db_claimed_at: Option<i64>,
    pub db_claimed_signature: Option<String>,
}

/// Vault funding requirements
#[derive(Debug, Clone)]
pub struct VaultRequirement {
    pub cohort_name: String,
    pub vault_index: usize,
    pub required_tokens: u64,
}

/// Merkle proof data for claiming
#[derive(Debug, Clone)]
pub struct ClaimProof {
    pub claimant: Pubkey,
    pub cohort_name: String,
    pub merkle_proof: Vec<String>, // Hex-encoded proof elements
    pub entitlements: u64,
}

/// Unified database interface for campaign operations
pub struct CampaignDatabase {
    conn: Connection,
}

impl CampaignDatabase {
    /// Open an existing database file (read-only operations)
    pub fn open(path: &Path) -> DbResult<Self> {
        if !path.exists() {
            return Err(DbError::InvalidConfig(format!(
                "Database file does not exist: {}",
                path.display()
            )));
        }

        let conn = Connection::open(path)
            .map_err(|e| DbError::Connection(format!("Failed to open database: {}", e)))?;

        // Verify it has the expected schema
        let db = Self { conn };
        if !db.verify_schema()? {
            return Err(DbError::InvalidConfig(format!(
                "Database file has invalid schema: {}",
                path.display()
            )));
        }

        Ok(db)
    }

    /// Create a new in-memory database with initialized schema
    /// Use save_to_file() to persist when ready
    pub fn create_in_memory() -> DbResult<Self> {
        let conn = Connection::open(":memory:").map_err(|e| {
            DbError::Connection(format!("Failed to create in-memory database: {}", e))
        })?;

        initialize_database(&conn)?;

        Ok(Self { conn })
    }

    /// Create a new database file, overwriting if it exists
    /// For when you explicitly want to create a file-based database
    pub fn create_file(path: &Path, overwrite: bool) -> DbResult<Self> {
        if path.exists() && !overwrite {
            return Err(DbError::InvalidConfig(format!(
                "Database file already exists (use overwrite=true to replace): {}",
                path.display()
            )));
        }

        // Remove existing file if overwriting
        if path.exists() && overwrite {
            std::fs::remove_file(path).map_err(|e| {
                DbError::Connection(format!("Failed to remove existing file: {}", e))
            })?;
        }

        let conn = Connection::open(path)
            .map_err(|e| DbError::Connection(format!("Failed to create database file: {}", e)))?;

        initialize_database(&conn)?;

        Ok(Self { conn })
    }

    /// Save in-memory database to a file
    pub fn save_to_file(&self, path: &Path, overwrite: bool) -> DbResult<()> {
        if path.exists() && !overwrite {
            return Err(DbError::InvalidConfig(format!(
                "File already exists (use overwrite=true to replace): {}",
                path.display()
            )));
        }

        // Remove existing file if overwriting
        if path.exists() && overwrite {
            std::fs::remove_file(path).map_err(|e| {
                DbError::Connection(format!("Failed to remove existing file: {}", e))
            })?;
        }

        // Create file connection
        let mut file_conn = Connection::open(path)
            .map_err(|e| DbError::Connection(format!("Failed to create output file: {}", e)))?;

        // Use SQLite backup API to copy in-memory DB to file
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut file_conn)
            .map_err(|e| DbError::Connection(format!("Failed to create backup: {}", e)))?;

        backup
            .run_to_completion(5, std::time::Duration::from_millis(250), None)
            .map_err(|e| DbError::Connection(format!("Failed to save database: {}", e)))?;

        Ok(())
    }

    /// Check if database has proper schema
    pub fn verify_schema(&self) -> DbResult<bool> {
        check_schema(&self.conn)
    }

    /// Get underlying connection for advanced operations
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Read basic campaign information
    pub fn read_campaign_info(&self) -> DbResult<CampaignInfo> {
        let mut stmt = self
            .conn
            .prepare("SELECT fingerprint, mint, admin FROM campaign LIMIT 1")
            .map_err(|e| DbError::Database(e))?;

        let mut rows = stmt
            .query_map([], |row| {
                let fingerprint_hex: String = row.get(0)?;
                let mint_str: String = row.get(1)?;
                let admin_str: String = row.get(2)?;
                Ok((fingerprint_hex, mint_str, admin_str))
            })
            .map_err(|e| DbError::Database(e))?;

        if let Some(row) = rows.next() {
            let (fingerprint_hex, mint_str, admin_str) = row.map_err(|e| DbError::Database(e))?;

            let fingerprint_bytes = hex::decode(fingerprint_hex)
                .map_err(|e| DbError::Serialization(format!("Invalid fingerprint hex: {}", e)))?;
            let fingerprint: [u8; 32] = fingerprint_bytes
                .try_into()
                .map_err(|_| DbError::Serialization("Fingerprint must be 32 bytes".to_string()))?;

            let mint = Pubkey::from_str(&mint_str)
                .map_err(|e| DbError::InvalidPubkey(format!("Invalid mint pubkey: {}", e)))?;

            let admin = Pubkey::from_str(&admin_str)
                .map_err(|e| DbError::InvalidPubkey(format!("Invalid admin pubkey: {}", e)))?;

            Ok(CampaignInfo {
                fingerprint,
                mint,
                admin,
            })
        } else {
            Err(DbError::InvalidConfig(
                "No campaign data found in database".to_string(),
            ))
        }
    }

    /// Read all cohort information
    pub fn read_cohorts(&self) -> DbResult<Vec<CohortInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT cohort_name, merkle_root, amount_per_entitlement FROM cohorts")
            .map_err(|e| DbError::Database(e))?;

        let cohort_rows = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let merkle_root_hex: String = row.get(1)?;
                let amount_per_entitlement: u64 = row.get(2)?;
                Ok((name, merkle_root_hex, amount_per_entitlement))
            })
            .map_err(|e| DbError::Database(e))?;

        let mut cohorts = Vec::new();

        for row in cohort_rows {
            let (name, merkle_root_hex, amount_per_entitlement) =
                row.map_err(|e| DbError::Database(e))?;

            let merkle_root_bytes = hex::decode(merkle_root_hex)
                .map_err(|e| DbError::Serialization(format!("Invalid merkle root hex: {}", e)))?;
            let merkle_root: [u8; 32] = merkle_root_bytes
                .try_into()
                .map_err(|_| DbError::Serialization("Merkle root must be 32 bytes".to_string()))?;

            // Get vaults for this cohort
            let mut vault_stmt = self
                .conn
                .prepare(
                    "SELECT vault_pubkey FROM vaults WHERE cohort_name = ? ORDER BY vault_index",
                )
                .map_err(|e| DbError::Database(e))?;

            let vault_rows = vault_stmt
                .query_map([&name], |row| {
                    let vault_str: String = row.get(0)?;
                    Ok(vault_str)
                })
                .map_err(|e| DbError::Database(e))?;

            let mut vaults = Vec::new();
            for vault_row in vault_rows {
                let vault_str = vault_row.map_err(|e| DbError::Database(e))?;
                let vault_pubkey = Pubkey::from_str(&vault_str)
                    .map_err(|e| DbError::InvalidPubkey(format!("Invalid vault pubkey: {}", e)))?;
                vaults.push(vault_pubkey);
            }

            let vault_count = vaults.len();
            cohorts.push(CohortInfo {
                name,
                merkle_root,
                amount_per_entitlement,
                vaults,
                vault_count,
            });
        }

        Ok(cohorts)
    }

    /// Get claimant eligibility across all cohorts
    pub fn read_claimant_eligibility(&self, claimant: &Pubkey) -> DbResult<Vec<EligibilityInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT 
                c.cohort_name,
                c.entitlements,
                c.claimed_at,
                c.claimed_signature,
                h.amount_per_entitlement,
                h.merkle_root
             FROM claimants c
             JOIN cohorts h ON c.cohort_name = h.cohort_name
             WHERE c.claimant = ?
             ORDER BY c.cohort_name",
            )
            .map_err(|e| DbError::Database(e))?;

        let rows = stmt
            .query_map([claimant.to_string()], |row| {
                let cohort_name: String = row.get(0)?;
                let entitlements: u64 = row.get(1)?;
                let claimed_at: Option<i64> = row.get(2)?;
                let claimed_signature: Option<String> = row.get(3)?;
                let amount_per_entitlement: u64 = row.get(4)?;
                let merkle_root_hex: String = row.get(5)?;

                Ok((
                    cohort_name,
                    entitlements,
                    claimed_at,
                    claimed_signature,
                    amount_per_entitlement,
                    merkle_root_hex,
                ))
            })
            .map_err(|e| DbError::Database(e))?;

        let mut eligibility = Vec::new();
        for row in rows {
            let (
                cohort_name,
                entitlements,
                claimed_at,
                claimed_signature,
                amount_per_entitlement,
                merkle_root_hex,
            ) = row.map_err(|e| DbError::Database(e))?;

            // Parse merkle root
            let merkle_root_bytes = hex::decode(merkle_root_hex)
                .map_err(|e| DbError::Serialization(format!("Invalid merkle root hex: {}", e)))?;
            let cohort_merkle_root: [u8; 32] = merkle_root_bytes
                .try_into()
                .map_err(|_| DbError::Serialization("Merkle root must be 32 bytes".to_string()))?;

            let total_tokens = entitlements * amount_per_entitlement;
            let already_claimed = claimed_at.is_some();

            eligibility.push(EligibilityInfo {
                cohort_name,
                cohort_merkle_root,
                entitlements,
                amount_per_entitlement,
                total_tokens,
                db_claimed: already_claimed,
                db_claimed_at: claimed_at,
                db_claimed_signature: claimed_signature,
            });
        }

        Ok(eligibility)
    }

    /// Read vault funding requirements
    pub fn read_vault_requirements(&self) -> DbResult<Vec<VaultRequirement>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT cohort_name, vault_index, required_tokens 
             FROM vaults ORDER BY cohort_name, vault_index",
            )
            .map_err(|e| DbError::Database(e))?;

        let vault_rows = stmt
            .query_map([], |row| {
                let cohort_name: String = row.get(0)?;
                let vault_index: i64 = row.get(1)?;
                let required_tokens: u64 = row.get(2)?;
                Ok((cohort_name, vault_index, required_tokens))
            })
            .map_err(|e| DbError::Database(e))?;

        let mut vault_requirements = Vec::new();
        for row in vault_rows {
            let (cohort_name, vault_index, required_tokens) =
                row.map_err(|e| DbError::Database(e))?;

            vault_requirements.push(VaultRequirement {
                cohort_name,
                vault_index: vault_index as usize,
                required_tokens,
            });
        }

        Ok(vault_requirements)
    }

    /// Calculate total tokens required across all vaults (convenience method)
    pub fn calculate_total_funding_required(&self) -> DbResult<u64> {
        let vault_requirements = self.read_vault_requirements()?;
        Ok(vault_requirements.iter().map(|v| v.required_tokens).sum())
    }

    /// Get merkle proof for a claimant in a specific cohort
    pub fn read_merkle_proof(&self, claimant: &Pubkey, cohort_name: &str) -> DbResult<ClaimProof> {
        // Get proof data from claimants table
        let mut stmt = self.conn.prepare(
            "SELECT merkle_proof, entitlements FROM claimants WHERE claimant = ? AND cohort_name = ?"
        ).map_err(|e| DbError::Database(e))?;

        let mut rows = stmt
            .query_map(params![claimant.to_string(), cohort_name], |row| {
                let proof_hex: String = row.get(0)?;
                let entitlements: u64 = row.get(1)?;
                Ok((proof_hex, entitlements))
            })
            .map_err(|e| DbError::Database(e))?;

        if let Some(row) = rows.next() {
            let (proof_hex, entitlements) = row.map_err(|e| DbError::Database(e))?;

            // Parse comma-separated hex proof
            let merkle_proof: Vec<String> = proof_hex
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            Ok(ClaimProof {
                claimant: *claimant,
                cohort_name: cohort_name.to_string(),
                merkle_proof,
                entitlements,
            })
        } else {
            Err(DbError::InvalidConfig(format!(
                "No merkle proof found for claimant {} in cohort {}",
                claimant, cohort_name
            )))
        }
    }

    /// Update campaign deployment status
    pub fn update_campaign_deployment(&mut self, signature: &str) -> DbResult<()> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| DbError::Transaction(format!("Failed to start transaction: {}", e)))?;

        tx.execute(
            "UPDATE campaign SET deployed_signature = ?, deployed_at = datetime('now') WHERE rowid = 1",
            params![signature],
        ).map_err(|e| DbError::Database(e))?;

        tx.commit()
            .map_err(|e| DbError::Transaction(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Update vault funding status
    pub fn update_vault_funding(
        &mut self,
        cohort_name: &str,
        vault_index: usize,
        signature: &str,
        amount: u64,
    ) -> DbResult<()> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| DbError::Transaction(format!("Failed to start transaction: {}", e)))?;

        tx.execute(
            "UPDATE vaults SET funded_signature = ?, funded_at = datetime('now'), funded_amount = ? 
             WHERE cohort_name = ? AND vault_index = ?",
            params![signature, amount, cohort_name, vault_index as i64],
        ).map_err(|e| DbError::Database(e))?;

        tx.commit()
            .map_err(|e| DbError::Transaction(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Update claim status
    pub fn update_claim_status(
        &mut self,
        claimant: &Pubkey,
        cohort_name: &str,
        signature: &str,
    ) -> DbResult<()> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| DbError::Transaction(format!("Failed to start transaction: {}", e)))?;

        tx.execute(
            "UPDATE claimants SET claimed_at = datetime('now'), claimed_signature = ? 
             WHERE claimant = ? AND cohort_name = ?",
            params![signature, claimant.to_string(), cohort_name],
        )
        .map_err(|e| DbError::Database(e))?;

        tx.commit()
            .map_err(|e| DbError::Transaction(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Insert campaign data (for use by SDK during compilation)
    pub fn insert_campaign(
        &mut self,
        fingerprint: [u8; 32],
        mint: Pubkey,
        admin: Pubkey,
    ) -> DbResult<()> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| DbError::Transaction(format!("Failed to start transaction: {}", e)))?;

        tx.execute(
            "INSERT INTO campaign (fingerprint, mint, admin) VALUES (?, ?, ?)",
            params![
                hex::encode(fingerprint),
                mint.to_string(),
                admin.to_string()
            ],
        )
        .map_err(|e| DbError::Database(e))?;

        tx.commit()
            .map_err(|e| DbError::Transaction(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Insert cohort data (for use by SDK during compilation)
    pub fn insert_cohort(
        &mut self,
        name: &str,
        merkle_root: [u8; 32],
        amount_per_entitlement: u64,
    ) -> DbResult<()> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| DbError::Transaction(format!("Failed to start transaction: {}", e)))?;

        tx.execute(
            "INSERT INTO cohorts (cohort_name, merkle_root, amount_per_entitlement) VALUES (?, ?, ?)",
            params![
                name,
                hex::encode(merkle_root),
                amount_per_entitlement
            ],
        ).map_err(|e| DbError::Database(e))?;

        tx.commit()
            .map_err(|e| DbError::Transaction(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Insert claimant data (for use by SDK during compilation)
    pub fn insert_claimant(
        &mut self,
        claimant: Pubkey,
        cohort_name: &str,
        entitlements: u64,
        merkle_proof: &str,
    ) -> DbResult<()> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| DbError::Transaction(format!("Failed to start transaction: {}", e)))?;

        tx.execute(
            "INSERT INTO claimants (claimant, cohort_name, entitlements, merkle_proof) VALUES (?, ?, ?, ?)",
            params![
                claimant.to_string(),
                cohort_name,
                entitlements,
                merkle_proof
            ],
        ).map_err(|e| DbError::Database(e))?;

        tx.commit()
            .map_err(|e| DbError::Transaction(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Insert vault data (for use by SDK during compilation)
    pub fn insert_vault(
        &mut self,
        cohort_name: &str,
        vault_index: usize,
        vault_pubkey: Pubkey,
        required_tokens: u64,
    ) -> DbResult<()> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| DbError::Transaction(format!("Failed to start transaction: {}", e)))?;

        tx.execute(
            "INSERT INTO vaults (cohort_name, vault_index, vault_pubkey, required_tokens) VALUES (?, ?, ?, ?)",
            params![
                cohort_name,
                vault_index as i64,
                vault_pubkey.to_string(),
                required_tokens
            ],
        ).map_err(|e| DbError::Database(e))?;

        tx.commit()
            .map_err(|e| DbError::Transaction(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }
}

/*!
# Campaign Database Operations

This module provides a simple workflow for campaign database operations:

1. **Create** a new writeable database with `new_writeable_campaign_db()`
2. **Populate** it with campaign data (CSVs, merkle trees, proofs, etc.)
3. **Backup** it to a file with `backup_campaign_db()` for distribution
4. **Open** saved databases in read-only mode with `open_readonly_campaign_db()`

The writeable database is backed by a temporary file (implementation detail) and supports
all normal database operations. Once backed up, the database file is marked read-only and
should only be opened for querying.
*/

use prism_protocol_migrations::MigratorTrait as _;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::path::Path;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    SeaOrm(#[from] sea_orm::DbErr),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Open a saved campaign database in read-only mode
///
/// This opens a previously saved campaign database for querying. The database
/// should have been created with `backup_campaign_db()`.
pub async fn open_readonly_campaign_db<P: AsRef<Path>>(path: P) -> Result<DatabaseConnection> {
    let path = path.as_ref();

    let mut url = Url::parse("sqlite:///").expect("sqlite:/// is a valid URL base");
    url.set_path(&path.to_string_lossy());
    url.set_query(Some("mode=ro"));

    let conn = Database::connect(url.as_str()).await?;
    Ok(conn)
}

/// Create a new writeable campaign database
///
/// This creates a fresh, empty database with all migrations applied, ready for populating
/// with campaign data. The database is backed by a temporary file but this is an
/// implementation detail - callers should treat it as an ephemeral writeable space.
pub async fn new_writeable_campaign_db() -> Result<DatabaseConnection> {
    // Implementation note: We use a temporary file rather than :memory: because
    // VACUUM INTO (used by backup_campaign_db) doesn't work reliably with in-memory databases
    let temp = tempfile::NamedTempFile::new()?;
    let path = temp.path().to_string_lossy();

    let mut url = Url::parse("sqlite:///").expect("sqlite:/// is a valid URL base");
    url.set_path(&path);
    url.set_query(Some("mode=rw"));

    let conn = Database::connect(url.as_str()).await?;
    prism_protocol_migrations::Migrator::up(&conn, None).await?;

    // Keep the temp file alive by forgetting it (cleaned up when process exits)
    std::mem::forget(temp);

    Ok(conn)
}

/// Backup a campaign database to a file for distribution
///
/// This backs up any campaign database to a compact, read-only file suitable for distribution.
/// The file will be marked read-only after creation to prevent accidental modification.
///
/// # Errors
///
/// Returns an error if the target file already exists.
pub async fn backup_campaign_db<P: AsRef<Path>>(conn: &DatabaseConnection, path: P) -> Result<()> {
    let path = path.as_ref();

    if path.exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Database file already exists: {}", path.display()),
        )));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Use SQLite's VACUUM INTO to create a compact copy
    let path_str = path.to_string_lossy();
    let vacuum_stmt = sea_orm::Statement::from_string(
        sea_orm::DbBackend::Sqlite,
        format!("VACUUM INTO '{}'", path_str.replace("'", "''")),
    );

    conn.execute(vacuum_stmt).await?;

    // Verify the file was created and mark it read-only
    if !path.exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Database file was not created",
        )));
    }

    // Mark the file read-only to prevent accidental modification
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(path, perms)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DbBackend, Statement};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_new_writeable_campaign_db() {
        let conn = new_writeable_campaign_db().await.unwrap();
        assert!(conn.get_database_backend() == DbBackend::Sqlite);

        // Verify schema was applied by checking table exists
        let table_check_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table' AND name='campaign_csv_rows'"
                .to_string(),
        );
        let result = conn.query_one(table_check_sql).await.unwrap();
        assert!(
            result.is_some(),
            "Migrations should have created campaign tables"
        );
    }

    #[tokio::test]
    async fn test_campaign_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("campaign.db");

        // Step 1: Create writeable database
        let conn = new_writeable_campaign_db().await.unwrap();

        // Step 2: Populate with sample campaign data
        let insert_sql = Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO campaign_csv_rows (cohort, claimant, entitlements) 
             VALUES ('test_cohort', 'So11111111111111111111111111111111111111112', 100)"
                .to_string(),
        );
        conn.execute(insert_sql).await.unwrap();

        // Step 3: Backup to file for distribution
        backup_campaign_db(&conn, &db_path).await.unwrap();

        // Verify file was created and is read-only
        assert!(db_path.exists());
        assert!(db_path.metadata().unwrap().len() > 0);
        assert!(db_path.metadata().unwrap().permissions().readonly());

        // Step 4: Open read-only and verify data persisted
        let readonly_conn = open_readonly_campaign_db(&db_path).await.unwrap();

        let select_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT cohort, claimant FROM campaign_csv_rows WHERE cohort = 'test_cohort'"
                .to_string(),
        );

        let result = readonly_conn.query_one(select_sql).await.unwrap();
        assert!(result.is_some());

        let row = result.unwrap();
        let cohort: String = row.try_get("", "cohort").unwrap();
        let claimant: String = row.try_get("", "claimant").unwrap();

        assert_eq!(cohort, "test_cohort");
        assert_eq!(claimant, "So11111111111111111111111111111111111111112");
    }

    #[tokio::test]
    async fn test_save_to_existing_file_error() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("existing.db");

        // Create a file that already exists
        std::fs::write(&db_path, "dummy content").unwrap();

        let conn = new_writeable_campaign_db().await.unwrap();

        // Try to backup to existing file - should error
        let result = backup_campaign_db(&conn, &db_path).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            Error::Io(io_err) => {
                assert!(io_err.to_string().contains("already exists"));
            }
            other => panic!("Expected IO error for existing file, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_path_with_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test file with spaces & 'quotes'.db");

        let conn = new_writeable_campaign_db().await.unwrap();

        // Insert test data
        let insert_sql = Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO campaign_csv_rows (cohort, claimant, entitlements) 
             VALUES ('test_cohort', 'So11111111111111111111111111111111111111112', 100)"
                .to_string(),
        );
        conn.execute(insert_sql).await.unwrap();

        // Backup and open with problematic path
        backup_campaign_db(&conn, &db_path).await.unwrap();
        assert!(db_path.exists());

        let readonly_conn = open_readonly_campaign_db(&db_path).await.unwrap();

        // Verify data persisted
        let select_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT cohort FROM campaign_csv_rows WHERE cohort = 'test_cohort'".to_string(),
        );
        let result = readonly_conn.query_one(select_sql).await.unwrap();
        assert!(result.is_some());
    }
}

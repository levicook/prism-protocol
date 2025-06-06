use prism_protocol_csvs::CampaignCsvRow;
use prism_protocol_entities::campaign_csv_rows::{ActiveModel, Entity};
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait as _};

use super::CompilerResult;

pub(super) async fn import_campaign_csv_rows(
    db: &DatabaseConnection,
    campaign_csv_rows: &[CampaignCsvRow],
) -> CompilerResult<()> {
    if campaign_csv_rows.is_empty() {
        return Ok(());
    }

    let models = campaign_csv_rows
        .iter()
        .enumerate()
        .map(|(id, row)| {
            Ok(ActiveModel {
                id: Set(id.try_into()?),
                cohort: Set(row.cohort.clone()),
                claimant: Set(row.claimant.to_string()),
                entitlements: Set(row.entitlements.try_into()?),
            })
        })
        .collect::<CompilerResult<Vec<_>>>()?;

    Entity::insert_many(models).exec(db).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{campaign_compiler::CompilerError, campaign_database::new_writeable_campaign_db};
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    // Helper function to create test campaign CSV data
    fn create_test_campaign_csv_rows() -> Vec<CampaignCsvRow> {
        vec![
            CampaignCsvRow {
                cohort: "early_adopters".to_string(),
                claimant: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
                entitlements: 100,
            },
            CampaignCsvRow {
                cohort: "early_adopters".to_string(),
                claimant: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
                entitlements: 200,
            },
            CampaignCsvRow {
                cohort: "power_users".to_string(),
                claimant: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                entitlements: 50,
            },
        ]
    }

    #[tokio::test]
    async fn test_import_campaign_csv_rows() {
        let db = new_writeable_campaign_db().await.unwrap();
        let test_rows = create_test_campaign_csv_rows();

        // Import the test data
        import_campaign_csv_rows(&db, &test_rows).await.unwrap();

        // Verify data was inserted correctly
        let select_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT id, cohort, claimant, entitlements FROM campaign_csv_rows ORDER BY id"
                .to_string(),
        );

        let results = db.query_all(select_sql).await.unwrap();
        assert_eq!(results.len(), 3);

        // Check first row
        let row = &results[0];
        assert_eq!(row.try_get::<i32>("", "id").unwrap(), 0);
        assert_eq!(
            row.try_get::<String>("", "cohort").unwrap(),
            "early_adopters"
        );
        assert_eq!(
            row.try_get::<String>("", "claimant").unwrap(),
            "So11111111111111111111111111111111111111112"
        );
        assert_eq!(row.try_get::<i32>("", "entitlements").unwrap(), 100);

        // Check second row
        let row = &results[1];
        assert_eq!(row.try_get::<i32>("", "id").unwrap(), 1);
        assert_eq!(
            row.try_get::<String>("", "cohort").unwrap(),
            "early_adopters"
        );
        assert_eq!(
            row.try_get::<String>("", "claimant").unwrap(),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        );
        assert_eq!(row.try_get::<i32>("", "entitlements").unwrap(), 200);

        // Check third row
        let row = &results[2];
        assert_eq!(row.try_get::<i32>("", "id").unwrap(), 2);
        assert_eq!(row.try_get::<String>("", "cohort").unwrap(), "power_users");
        assert_eq!(
            row.try_get::<String>("", "claimant").unwrap(),
            "11111111111111111111111111111111"
        );
        assert_eq!(row.try_get::<i32>("", "entitlements").unwrap(), 50);
    }

    #[tokio::test]
    async fn test_import_empty_campaign_csv_rows() {
        let db = new_writeable_campaign_db().await.unwrap();
        let empty_rows: Vec<CampaignCsvRow> = vec![];

        // Import empty data should succeed
        import_campaign_csv_rows(&db, &empty_rows).await.unwrap();

        // Verify no data was inserted
        let count_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT COUNT(*) as count FROM campaign_csv_rows".to_string(),
        );

        let result = db.query_one(count_sql).await.unwrap().unwrap();
        let count: i32 = result.try_get("", "count").unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_import_large_entitlements() {
        let db = new_writeable_campaign_db().await.unwrap();

        // Test with u64::MAX which should fail to convert to i32
        let large_entitlements_row = vec![CampaignCsvRow {
            cohort: "whales".to_string(),
            claimant: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            entitlements: u64::MAX, // Test maximum value - should fail conversion
        }];

        // This should fail due to TryFromIntError when converting u64::MAX to i32
        let result = import_campaign_csv_rows(&db, &large_entitlements_row).await;
        assert!(result.is_err());

        // Verify it's the expected error type
        match result.unwrap_err() {
            CompilerError::TryFromInt(_) => {
                // Expected - u64::MAX doesn't fit in i32
            }
            other => panic!("Expected TryFromIntError, got: {:?}", other),
        }

        // Test with a value that does fit in i32
        let valid_large_entitlements = vec![CampaignCsvRow {
            cohort: "whales".to_string(),
            claimant: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            entitlements: i32::MAX as u64, // This should work
        }];

        import_campaign_csv_rows(&db, &valid_large_entitlements)
            .await
            .unwrap();

        // Verify the valid large entitlement was stored correctly
        let select_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT entitlements FROM campaign_csv_rows".to_string(),
        );

        let result = db.query_one(select_sql).await.unwrap().unwrap();
        let stored_entitlements: i32 = result.try_get("", "entitlements").unwrap();
        assert_eq!(stored_entitlements, i32::MAX);
    }
}

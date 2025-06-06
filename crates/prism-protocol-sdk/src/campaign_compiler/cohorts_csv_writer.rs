use prism_protocol_csvs::CohortsCsvRow;
use prism_protocol_entities::cohorts_csv_rows::{ActiveModel, Entity};
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait as _};

use super::CompilerResult;

pub(super) async fn import_cohorts_csv_rows(
    db: &DatabaseConnection,
    cohorts_csv_rows: &[CohortsCsvRow],
) -> CompilerResult<()> {
    if cohorts_csv_rows.is_empty() {
        return Ok(());
    }

    let models = cohorts_csv_rows
        .iter()
        .enumerate()
        .map(|(id, row)| {
            Ok(ActiveModel {
                id: Set(id.try_into()?),
                cohort: Set(row.cohort.clone()),
                share_percentage: Set(row.share_percentage.to_string()),
            })
        })
        .collect::<CompilerResult<Vec<_>>>()?;

    Entity::insert_many(models).exec(db).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign_database::new_writeable_campaign_db;
    use rust_decimal::Decimal;
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    // Helper function to create test cohorts CSV data
    fn create_test_cohorts_csv_rows() -> Vec<CohortsCsvRow> {
        vec![
            CohortsCsvRow {
                cohort: "early_adopters".to_string(),
                share_percentage: Decimal::from(70),
            },
            CohortsCsvRow {
                cohort: "power_users".to_string(),
                share_percentage: Decimal::from(30),
            },
        ]
    }

    #[tokio::test]
    async fn test_import_cohorts_csv_rows() {
        let db = new_writeable_campaign_db().await.unwrap();
        let test_rows = create_test_cohorts_csv_rows();

        // Import the test data
        import_cohorts_csv_rows(&db, &test_rows).await.unwrap();

        // Verify data was inserted correctly
        let select_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT id, cohort, share_percentage FROM cohorts_csv_rows ORDER BY id".to_string(),
        );

        let results = db.query_all(select_sql).await.unwrap();
        assert_eq!(results.len(), 2);

        // Check first row
        let row = &results[0];
        assert_eq!(row.try_get::<i32>("", "id").unwrap(), 0);
        assert_eq!(
            row.try_get::<String>("", "cohort").unwrap(),
            "early_adopters"
        );
        // Decimal is stored as REAL in SQLite
        let share_percentage: Decimal = row.try_get("", "share_percentage").unwrap();
        assert_eq!(share_percentage, Decimal::from(70));

        // Check second row
        let row = &results[1];
        assert_eq!(row.try_get::<i32>("", "id").unwrap(), 1);
        assert_eq!(row.try_get::<String>("", "cohort").unwrap(), "power_users");
        let share_percentage: Decimal = row.try_get("", "share_percentage").unwrap();
        assert_eq!(share_percentage, Decimal::from(30));
    }

    #[tokio::test]
    async fn test_import_empty_cohorts_csv_rows() {
        let db = new_writeable_campaign_db().await.unwrap();
        let empty_rows: Vec<CohortsCsvRow> = vec![];

        // Import empty data should succeed
        import_cohorts_csv_rows(&db, &empty_rows).await.unwrap();

        // Verify no data was inserted
        let count_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT COUNT(*) as count FROM cohorts_csv_rows".to_string(),
        );

        let result = db.query_one(count_sql).await.unwrap().unwrap();
        let count: i32 = result.try_get("", "count").unwrap();
        assert_eq!(count, 0);
    }
}

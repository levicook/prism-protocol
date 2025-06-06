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
        .map(|(row_index, row)| {
            let row_id = row_index + 1;
            Ok(ActiveModel {
                id: Set(row_id.try_into()?),
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
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

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

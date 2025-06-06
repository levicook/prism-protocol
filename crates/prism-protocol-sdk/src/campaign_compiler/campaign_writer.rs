use prism_protocol_entities::compiled_campaigns::{ActiveModel, Entity};
use rust_decimal::Decimal;
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait as _};
use solana_sdk::pubkey::Pubkey;

use super::{ClaimTreeType, CompilerResult};

pub(super) async fn import_campaign(
    db: &DatabaseConnection,
    campaign_address: Pubkey,
    campaign_admin: Pubkey,
    campaign_budget: Decimal,
    campaign_mint: Pubkey,
    mint_decimals: u8,
    claimants_per_vault: usize,
    claim_tree_type: ClaimTreeType,
) -> CompilerResult<()> {
    debug_assert!(
        campaign_address != Pubkey::default(),
        "address must be non-zero"
    );
    debug_assert!(
        campaign_admin != Pubkey::default(),
        "admin must be non-zero"
    );
    debug_assert!(
        campaign_budget > Decimal::ZERO,
        "budget must be greater than 0"
    );
    debug_assert!(campaign_mint != Pubkey::default(), "mint must be non-zero");
    debug_assert!(mint_decimals > 0, "mint_decimals must be greater than 0");
    debug_assert!(
        claimants_per_vault > 0,
        "claimants_per_vault must be greater than 0"
    );

    let model = ActiveModel {
        address: Set(campaign_address.to_string()),
        campaign_admin: Set(campaign_admin.to_string()),
        campaign_budget: Set(campaign_budget.to_string()),
        campaign_mint: Set(campaign_mint.to_string()),
        mint_decimals: Set(mint_decimals as i16), // infallible conversion
        claimants_per_vault: Set(claimants_per_vault.try_into()?),
        claim_tree_type: Set(claim_tree_type.to_string()),
    };

    Entity::insert(model).exec(db).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{campaign_compiler::CompilerError, campaign_database::new_writeable_campaign_db};
    use rust_decimal::Decimal;
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    // Helper function to create test campaign data
    fn create_test_campaign_params() -> (Pubkey, Pubkey, Decimal, Pubkey, u8, usize) {
        let address = Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap();
        let admin = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let budget = Decimal::from(1000);
        let mint = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let mint_decimals = 9;
        let claimants_per_vault = 100;

        (
            address,
            admin,
            budget,
            mint,
            mint_decimals,
            claimants_per_vault,
        )
    }

    #[tokio::test]
    async fn test_import_campaign_large_values() {
        let db = new_writeable_campaign_db().await.unwrap();

        let address = Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap();
        let admin = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let budget = Decimal::from(999999999); // Large budget
        let mint = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let mint_decimals = 18; // Maximum reasonable decimals
        let claimants_per_vault = usize::MAX; // This should fail conversion to u32

        // This should fail due to usize::MAX not fitting in u32
        let result = import_campaign(
            &db,
            address,
            admin,
            budget,
            mint,
            mint_decimals,
            claimants_per_vault,
            ClaimTreeType::V0,
        )
        .await;
        assert!(result.is_err());

        // Verify it's a conversion error
        match result.unwrap_err() {
            CompilerError::TryFromInt(_) => {
                // Expected - usize::MAX doesn't fit in u32
            }
            other => panic!("Expected TryFromIntError, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_import_campaign_max_valid_values() {
        let db = new_writeable_campaign_db().await.unwrap();

        let address = Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap();
        let admin = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let budget = Decimal::from(999999999);
        let mint = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let mint_decimals = u8::MAX; // Maximum u8 value
        let claimants_per_vault = i32::MAX as usize; // Maximum value that fits in i32

        // This should succeed
        import_campaign(
            &db,
            address,
            admin,
            budget,
            mint,
            mint_decimals,
            claimants_per_vault,
            ClaimTreeType::V0,
        )
        .await
        .unwrap();

        // Verify the large values were stored correctly
        let select_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT mint_decimals, claimants_per_vault FROM compiled_campaigns".to_string(),
        );

        let result = db.query_one(select_sql).await.unwrap().unwrap();
        assert_eq!(
            result.try_get::<i16>("", "mint_decimals").unwrap(),
            mint_decimals as i16
        );
        assert_eq!(
            result.try_get::<u32>("", "claimants_per_vault").unwrap(),
            claimants_per_vault as u32
        );
    }

    #[tokio::test]
    async fn test_import_campaign_decimal_precision() {
        let db = new_writeable_campaign_db().await.unwrap();

        let address = Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap();
        let admin = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let budget = Decimal::new(123456789, 3); // 123456.789 with 3 decimal places
        let mint = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let mint_decimals = 9;
        let claimants_per_vault = 50;

        import_campaign(
            &db,
            address,
            admin,
            budget,
            mint,
            mint_decimals,
            claimants_per_vault,
            ClaimTreeType::V0,
        )
        .await
        .unwrap();

        // Verify the decimal budget was stored correctly as string
        let select_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT campaign_budget FROM compiled_campaigns".to_string(),
        );

        let result = db.query_one(select_sql).await.unwrap().unwrap();
        let stored_budget_str = result.try_get::<String>("", "campaign_budget").unwrap();

        // Parse it back to verify precision was preserved
        let stored_budget: Decimal = stored_budget_str.parse().unwrap();
        assert_eq!(stored_budget, budget);
    }

    #[tokio::test]
    async fn test_import_campaign_count() {
        let db = new_writeable_campaign_db().await.unwrap();

        // Before import - should have 0 campaigns
        let count_sql = Statement::from_string(
            DbBackend::Sqlite,
            "SELECT COUNT(*) as count FROM compiled_campaigns".to_string(),
        );
        let result = db.query_one(count_sql.clone()).await.unwrap().unwrap();
        let count: i32 = result.try_get("", "count").unwrap();
        assert_eq!(count, 0);

        // Import campaign
        let (address, admin, budget, mint, mint_decimals, claimants_per_vault) =
            create_test_campaign_params();
        import_campaign(
            &db,
            address,
            admin,
            budget,
            mint,
            mint_decimals,
            claimants_per_vault,
            ClaimTreeType::V0,
        )
        .await
        .unwrap();

        // After import - should have 1 campaign
        let result = db.query_one(count_sql).await.unwrap().unwrap();
        let count: i32 = result.try_get("", "count").unwrap();
        assert_eq!(count, 1);
    }
}

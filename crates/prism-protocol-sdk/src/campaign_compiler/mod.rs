mod campaign_csv_writer;
mod campaign_writer;
mod claim_tree;
mod cohort_writer;
mod cohorts_csv_writer;
mod compiled_campaign_database;
mod compiler_error;

use {
    crate::{new_writeable_campaign_db, AddressFinder},
    prism_protocol_csvs::{validate_csv_consistency, CampaignCsvRow, CohortsCsvRow},
    rust_decimal::Decimal,
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer as _},
};

pub use {
    claim_tree::{ClaimTree, ClaimTreeType},
    compiled_campaign_database::*,
    compiler_error::{CompilerError, CompilerResult},
};

pub async fn compile_campaign(
    campaign_admin: Pubkey,   // campaign admin
    campaign_budget: Decimal, // total budget in human-readable tokens
    campaign_mint: Pubkey,    // SPL token mint for the campaign
    mint_decimals: u8, // number of decimals for the token mint (critical for budget allocation)
    campaign_csv_rows: &[CampaignCsvRow],
    cohorts_csv_rows: &[CohortsCsvRow],
    claimants_per_vault: usize, // ratio that determines rent -vs- claim contention
    claim_tree_type: ClaimTreeType,
) -> CompilerResult<CompiledCampaignDatabase> {
    validate_csv_consistency(campaign_csv_rows, cohorts_csv_rows)?; // fail fast if the csvs are invalid

    let campaign_keypair = Keypair::new();
    let campaign_address = campaign_keypair.pubkey();
    let address_finder = AddressFinder::new(campaign_admin, campaign_address, campaign_mint);

    let db = new_writeable_campaign_db().await?;

    campaign_csv_writer::import_campaign_csv_rows(&db, campaign_csv_rows).await?;
    cohorts_csv_writer::import_cohorts_csv_rows(&db, cohorts_csv_rows).await?;

    campaign_writer::import_campaign(
        &db,
        campaign_address,
        campaign_admin,
        campaign_budget,
        campaign_mint,
        mint_decimals,
        claimants_per_vault,
        claim_tree_type,
    )
    .await?;

    cohort_writer::import_cohorts(&address_finder, &db).await?;

    Ok(CompiledCampaignDatabase::new_with_keypair(
        address_finder,
        db,
        campaign_keypair,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::*;
    use solana_sdk::pubkey;

    // Test data matching the default fixture data
    fn test_campaign_csv_rows() -> Vec<CampaignCsvRow> {
        vec![
            // EarlyAdopters
            CampaignCsvRow {
                cohort: "EarlyAdopters".to_string(),
                claimant: pubkey!("11111111111111111111111111111112"), // early_adopter_1
                entitlements: 1,
            },
            CampaignCsvRow {
                cohort: "EarlyAdopters".to_string(),
                claimant: pubkey!("11111111111111111111111111111113"), // early_adopter_2
                entitlements: 2,
            },
            // Investors
            CampaignCsvRow {
                cohort: "Investors".to_string(),
                claimant: pubkey!("11111111111111111111111111111114"), // investor_1
                entitlements: 1,
            },
            CampaignCsvRow {
                cohort: "Investors".to_string(),
                claimant: pubkey!("11111111111111111111111111111115"), // investor_2
                entitlements: 2,
            },
        ]
    }

    fn test_cohorts_csv_rows() -> Vec<CohortsCsvRow> {
        vec![
            CohortsCsvRow {
                cohort: "EarlyAdopters".to_string(),
                share_percentage: Decimal::from(50), // 50% for easier math
            },
            CohortsCsvRow {
                cohort: "Investors".to_string(),
                share_percentage: Decimal::from(50), // 50% for easier math
            },
        ]
    }

    #[tokio::test]
    async fn test_compile_campaign_basic_functionality() {
        // Test that the basic compilation process works and populates all tables
        let campaign_admin = pubkey!("So11111111111111111111111111111111111111112");
        let campaign_budget = dec!(1000); // 1000 SOL for easy math
        let campaign_mint = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let mint_decimals = 9; // SOL-like
        let claimants_per_vault = 10;
        let claim_tree_type = ClaimTreeType::V0;

        let campaign_csv_rows = test_campaign_csv_rows();
        let cohorts_csv_rows = test_cohorts_csv_rows();

        // Compile the campaign
        let compiled_db = compile_campaign(
            campaign_admin,
            campaign_budget,
            campaign_mint,
            mint_decimals,
            &campaign_csv_rows,
            &cohorts_csv_rows,
            claimants_per_vault,
            claim_tree_type,
        )
        .await
        .expect("Campaign compilation should succeed");

        // ✅ Test 1: Campaign data is populated correctly
        let campaign = compiled_db.compiled_campaign().await;
        assert_eq!(campaign.address(), compiled_db.address_finder.campaign);
        assert_eq!(
            campaign.campaign_admin.parse::<Pubkey>().unwrap(),
            campaign_admin
        );
        assert_eq!(
            campaign.campaign_budget_human.parse::<Decimal>().unwrap(),
            campaign_budget
        );

        // ✅ Test 2: Human/token fields are populated correctly
        let expected_token_amount = (campaign_budget
            * Decimal::from(10_u64.pow(mint_decimals as u32)))
        .floor()
        .to_u64()
        .unwrap();
        assert_eq!(
            campaign.campaign_budget_token.parse::<u64>().unwrap(),
            expected_token_amount
        );

        // ✅ Test 3: Cohorts are created
        let cohorts = compiled_db.compiled_cohorts().await;
        assert_eq!(cohorts.len(), 2, "Should have 2 cohorts");

        // Find EarlyAdopters cohort
        let early_adopters = cohorts
            .iter()
            .find(|c| c.cohort_csv_row_id == 1)
            .expect("EarlyAdopters cohort should exist");

        // ✅ Test 4: Cohort human/token amounts are correct
        let expected_cohort_human = campaign_budget * (Decimal::from(50) / Decimal::from(100)); // 50%
        let expected_cohort_token = (expected_cohort_human
            * Decimal::from(10_u64.pow(mint_decimals as u32)))
        .floor()
        .to_u64()
        .unwrap();

        assert_eq!(
            early_adopters
                .cohort_budget_human
                .parse::<Decimal>()
                .unwrap(),
            expected_cohort_human
        );
        assert_eq!(
            early_adopters.cohort_budget_token.parse::<u64>().unwrap(),
            expected_cohort_token
        );

        // ✅ Test 5: Leaf data is populated (the key test!)
        let early_adopter_1 = pubkey!("11111111111111111111111111111112");
        let leaf = compiled_db
            .compiled_leaf_by_cohort_and_claimant(early_adopters.address(), early_adopter_1)
            .await;

        assert_eq!(leaf.claimant.parse::<Pubkey>().unwrap(), early_adopter_1);
        assert_eq!(leaf.entitlements.parse::<u64>().unwrap(), 1);

        // ✅ Test 6: Proof data is populated
        let proofs = compiled_db
            .compiled_proofs_by_claimant(early_adopter_1)
            .await;
        assert!(
            !proofs.is_empty(),
            "Should have proof data for early_adopter_1"
        );

        // ✅ Test 7: Vault data is populated with human/token amounts
        let vaults = compiled_db.compiled_vaults().await;
        assert!(!vaults.is_empty(), "Should have vault data");

        let vault = &vaults[0];
        // Verify both human and token amounts exist
        assert!(!vault.vault_budget_human.is_empty());
        assert!(!vault.vault_budget_token.is_empty());
        assert!(!vault.amount_per_entitlement_human.is_empty());
        assert!(!vault.amount_per_entitlement_token.is_empty());

        println!("✅ Campaign compilation test passed! All data populated correctly.");
    }

    #[tokio::test]
    async fn test_compile_campaign_leaf_lookup_precision() {
        // Specifically test the scenario that's failing in the main test
        let campaign_admin = pubkey!("So11111111111111111111111111111111111111112");
        let campaign_budget = dec!(1_000_000_000); // 1B tokens (matching test)
        let campaign_mint = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let mint_decimals = 9;
        let claimants_per_vault = 10;

        let campaign_csv_rows = test_campaign_csv_rows();
        let cohorts_csv_rows = test_cohorts_csv_rows();

        let compiled_db = compile_campaign(
            campaign_admin,
            campaign_budget,
            campaign_mint,
            mint_decimals,
            &campaign_csv_rows,
            &cohorts_csv_rows,
            claimants_per_vault,
            ClaimTreeType::V1, // Test V1 like the main test
        )
        .await
        .expect("Campaign compilation should succeed");

        // Test every claimant can be found
        for csv_row in &campaign_csv_rows {
            let cohorts = compiled_db.compiled_cohorts().await;
            let cohort = cohorts
                .iter()
                .find(|c| {
                    // Match by cohort name (this is the tricky part)
                    let csv_cohort_row = test_cohorts_csv_rows()
                        .iter()
                        .position(|r| r.cohort == csv_row.cohort)
                        .map(|pos| pos + 1); // 1-indexed

                    csv_cohort_row == Some(c.cohort_csv_row_id as usize)
                })
                .expect(&format!("Cohort for {} should exist", csv_row.cohort));

            // This is the exact call that's failing in the main test
            let leaf = compiled_db
                .compiled_leaf_by_cohort_and_claimant(cohort.address(), csv_row.claimant)
                .await;

            assert_eq!(leaf.claimant.parse::<Pubkey>().unwrap(), csv_row.claimant);
            assert_eq!(
                leaf.entitlements.parse::<u64>().unwrap(),
                csv_row.entitlements
            );

            println!(
                "✅ Found leaf for claimant {} in cohort {}",
                csv_row.claimant, csv_row.cohort
            );
        }

        println!("✅ All claimants can be found! Leaf lookup works correctly.");
    }

    #[tokio::test]
    async fn test_compile_campaign_human_token_precision() {
        // Test precision of human/token conversions with difficult numbers
        let campaign_admin = pubkey!("So11111111111111111111111111111111111111112");
        let campaign_budget = dec!(33.333333333); // Repeating decimal
        let campaign_mint = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let mint_decimals = 9;

        let campaign_csv_rows = vec![CampaignCsvRow {
            cohort: "Test".to_string(),
            claimant: pubkey!("11111111111111111111111111111112"),
            entitlements: 3, // Will cause 33.333.../3 = 11.111... per entitlement
        }];

        let cohorts_csv_rows = vec![CohortsCsvRow {
            cohort: "Test".to_string(),
            share_percentage: Decimal::from(100),
        }];

        let compiled_db = compile_campaign(
            campaign_admin,
            campaign_budget,
            campaign_mint,
            mint_decimals,
            &campaign_csv_rows,
            &cohorts_csv_rows,
            10,
            ClaimTreeType::V0,
        )
        .await
        .expect("Campaign compilation should succeed");

        // Check that human amounts are preserved as decimals
        let cohorts = compiled_db.compiled_cohorts().await;
        let cohort = &cohorts[0];
        let cohort_budget_human = cohort.cohort_budget_human.parse::<Decimal>().unwrap();
        assert_eq!(cohort_budget_human, campaign_budget);

        // Check vault precision
        let vaults = compiled_db.compiled_vaults().await;
        let vault = &vaults[0];

        let amount_per_entitlement_human = vault
            .amount_per_entitlement_human
            .parse::<Decimal>()
            .unwrap();
        let amount_per_entitlement_token =
            vault.amount_per_entitlement_token.parse::<u64>().unwrap();

        // Human amount should be exactly budget/entitlements
        let expected_human = campaign_budget / Decimal::from(3);
        assert_eq!(amount_per_entitlement_human, expected_human);

        // Token amount should be floor(human * 10^decimals)
        let expected_token = (expected_human * Decimal::from(10_u64.pow(9)))
            .floor()
            .to_u64()
            .unwrap();
        assert_eq!(amount_per_entitlement_token, expected_token);

        println!("✅ Human/token precision test passed!");
        println!("   Human per entitlement: {}", amount_per_entitlement_human);
        println!("   Token per entitlement: {}", amount_per_entitlement_token);
    }
}

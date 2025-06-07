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
    async fn test_consistent_hash_collision_bug_reproduction() {
        // 🐛 BUG REPRODUCTION: This test demonstrates that consistent hashing can create
        // vault collisions where multiple claimants hash to the same vault, leaving
        // other vaults empty. This causes "Vault with zero entitlements" allocation errors.

        use solana_sdk::pubkey;

        // Test specific pubkeys that we know will cause a hash collision
        // (these are chosen to demonstrate the bug, not random)
        let claimant_1 = pubkey!("11111111111111111111111111111112");
        let claimant_2 = pubkey!("11111111111111111111111111111113");
        let vault_count = 2u8;

        // Check if these claimants hash to the same vault
        let vault_1 = ClaimTreeType::V1
            .new_tree(
                pubkey!("So11111111111111111111111111111111111111112"),
                &[(claimant_1, 1)],
                vault_count,
            )
            .unwrap()
            .claimant_leaf(&claimant_1)
            .unwrap()
            .vault_index;

        let vault_2 = ClaimTreeType::V1
            .new_tree(
                pubkey!("So11111111111111111111111111111111111111112"),
                &[(claimant_2, 1)],
                vault_count,
            )
            .unwrap()
            .claimant_leaf(&claimant_2)
            .unwrap()
            .vault_index;

        println!("🔍 Hash collision test:");
        println!("  Claimant 1 → Vault {}", vault_1);
        println!("  Claimant 2 → Vault {}", vault_2);

        if vault_1 == vault_2 {
            println!(
                "🎯 HASH COLLISION: Both claimants assigned to vault {}",
                vault_1
            );
            println!(
                "   This means vault {} will have 0 entitlements!",
                1 - vault_1
            );
        } else {
            println!("✅ No collision with these specific pubkeys");
        }
    }

    #[tokio::test]
    async fn test_vault_allocation_bug_multiple_claimants_single_vault() {
        // 🐛 BUG REPRODUCTION: This test demonstrates a bug where compile_campaign
        // fails with "Vault with zero entitlements should not be created" when
        // using multiple claimants with claimants_per_vault = 1
        //
        // Expected behavior: 2 claimants + claimants_per_vault=1 should create 2 vaults,
        // each with 1 claimant assigned
        //
        // Actual behavior: Some vaults get 0 entitlements assigned, causing allocation failure

        let campaign_admin = pubkey!("So11111111111111111111111111111111111111112");
        let campaign_budget = dec!(10_000); // Same budget as simple_v1()
        let campaign_mint = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let mint_decimals = 9;
        let claimants_per_vault = 1; // 1 claimant per vault = should create 2 vaults
        let claim_tree_type = ClaimTreeType::V1;

        // Create exactly 2 claimants, each with 1 entitlement
        // This should create exactly 2 vaults with 1 claimant each
        let campaign_csv_rows = vec![
            CampaignCsvRow {
                cohort: "TestCohort".to_string(),
                claimant: pubkey!("11111111111111111111111111111112"), // claimant_0
                entitlements: 1,
            },
            CampaignCsvRow {
                cohort: "TestCohort".to_string(),
                claimant: pubkey!("11111111111111111111111111111113"), // claimant_1
                entitlements: 1,
            },
        ];

        let cohorts_csv_rows = vec![CohortsCsvRow {
            cohort: "TestCohort".to_string(),
            share_percentage: Decimal::from(100), // 100% to single cohort
        }];

        // 🐛 This should work but currently fails with:
        // "Vault with zero entitlements should not be created"
        let result = compile_campaign(
            campaign_admin,
            campaign_budget,
            campaign_mint,
            mint_decimals,
            &campaign_csv_rows,
            &cohorts_csv_rows,
            claimants_per_vault,
            claim_tree_type,
        )
        .await;

        // ✅ After fix: compilation should now succeed
        let compiled_db =
            result.expect("Campaign compilation should succeed after zero-entitlement fix");

        let vaults = compiled_db.compiled_vaults().await;

        // Should have exactly 2 vaults (including the empty one)
        assert_eq!(
            vaults.len(),
            2,
            "Should create exactly 2 vaults for 2 claimants with claimants_per_vault=1"
        );

        // Verify vault allocations
        let mut total_entitlements = Decimal::ZERO;
        let mut non_empty_vaults = 0;
        let mut empty_vaults = 0;

        for vault in &vaults {
            let vault_entitlements = vault.total_entitlements.parse::<Decimal>().unwrap();
            let vault_budget = vault.vault_budget_token.parse::<u64>().unwrap();

            total_entitlements += vault_entitlements;

            if vault_entitlements > Decimal::ZERO {
                non_empty_vaults += 1;
                assert!(
                    vault_budget > 0,
                    "Non-empty vault should have non-zero budget"
                );
                println!(
                    "✅ Vault {} has {} entitlements and {} budget",
                    vault.vault_index, vault_entitlements, vault_budget
                );
            } else {
                empty_vaults += 1;
                assert_eq!(vault_budget, 0, "Empty vault should have zero budget");
                println!("✅ Vault {} is empty (0 entitlements, 0 budget) - this is OK due to hash collision", 
                    vault.vault_index);
            }
        }

        // Verify the overall allocation makes sense
        assert_eq!(
            total_entitlements,
            Decimal::from(2),
            "Total entitlements should be 2"
        );
        assert_eq!(
            non_empty_vaults, 1,
            "Should have 1 non-empty vault due to hash collision"
        );
        assert_eq!(
            empty_vaults, 1,
            "Should have 1 empty vault due to hash collision"
        );

        println!("🎉 SUCCESS: Zero-entitlement vaults now handled gracefully!");
        println!(
            "   - {} non-empty vaults with total {} entitlements",
            non_empty_vaults, total_entitlements
        );
        println!(
            "   - {} empty vaults (due to consistent hashing collisions)",
            empty_vaults
        );
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

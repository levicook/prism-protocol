use std::{collections::HashMap, str::FromStr as _};

use anchor_lang::AnchorSerialize as _;
use prism_protocol::ClaimProof;
use prism_protocol_entities::{
    campaign_csv_rows, cohorts_csv_rows, compiled_campaigns, compiled_cohorts, compiled_leaves,
    compiled_proofs, compiled_vaults,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveValue::Set, ColumnTrait as _, DatabaseConnection, EntityTrait as _, PaginatorTrait as _,
    QueryFilter as _,
};
use solana_sdk::pubkey::Pubkey;

use crate::{
    budget_allocation::CohortAllocation,
    campaign_compiler::{CompilerError, CompilerResult},
    AddressFinder, BudgetAllocator, VaultAllocation,
};

use super::{ClaimTree, ClaimTreeType};

pub(super) async fn import_cohorts(
    address_finder: &AddressFinder,
    db: &DatabaseConnection,
) -> CompilerResult<()> {
    let Some(campaign) = compiled_campaigns::Entity::find()
        .filter(compiled_campaigns::Column::Address.eq(address_finder.campaign.to_string()))
        .one(db)
        .await?
    else {
        return Err(CompilerError::CampaignNotFound);
    };

    let budget_allocator = BudgetAllocator::new(
        campaign.campaign_budget_human.parse()?, //
        campaign.mint_decimals.try_into()?,
    )?;

    let claimants_per_vault = Decimal::from(campaign.claimants_per_vault);

    let claim_tree_type = ClaimTreeType::from_str(&campaign.claim_tree_type)
        .map_err(|e| CompilerError::InvalidClaimTreeType(e))?;

    let mut cohorts_csv_rows_pager = cohorts_csv_rows::Entity::find().paginate(db, 100);
    while let Some(cohorts_csv_rows) = cohorts_csv_rows_pager.fetch_and_next().await? {
        let mut cohorts = Vec::new();
        let mut leaves = Vec::new();
        let mut proofs = Vec::new();
        let mut vaults = Vec::new();

        for cohort_csv_row in cohorts_csv_rows {
            let (compiled_cohort, cohort_metadata) = build_compiled_cohort_and_metadata(
                &address_finder,
                db,
                &cohort_csv_row,
                &budget_allocator,
                claimants_per_vault,
                &claim_tree_type,
            )
            .await?;

            // 🔍 Debug assertion: Each cohort should have claimants
            debug_assert!(
                !cohort_metadata.claimant_entitlements.is_empty(),
                "Cohort '{}' should have at least one claimant",
                cohort_csv_row.cohort
            );

            // map of claim_leaf.vault_index -> total_entitlements
            let mut vault_total_entitlements: HashMap<u8, Decimal> = HashMap::new();

            for (claimant, entitlements) in &cohort_metadata.claimant_entitlements {
                let claim_leaf = cohort_metadata.claim_tree.claimant_leaf(claimant)?;
                let claim_proof = cohort_metadata.claim_tree.claimant_proof(claimant)?;

                // 🔍 Debug assertions: Validate claim tree data consistency
                debug_assert!(
                    claim_leaf.vault_index < cohort_metadata.vault_count,
                    "Vault index {} should be less than vault count {}",
                    claim_leaf.vault_index,
                    cohort_metadata.vault_count
                );
                debug_assert!(*entitlements > 0, "Entitlements should be greater than 0");

                vault_total_entitlements
                    .entry(claim_leaf.vault_index)
                    .and_modify(|e| *e += Decimal::from(*entitlements))
                    .or_insert(Decimal::from(*entitlements));

                let merkle_proof = match claim_proof {
                    ClaimProof::V0(proof) => hex::encode(proof.try_to_vec()?),
                    ClaimProof::V1(proof) => hex::encode(proof.try_to_vec()?),
                };

                // 🔍 Debug assertion: Validate merkle proof was generated
                debug_assert!(!merkle_proof.is_empty(), "Merkle proof should not be empty");

                leaves.push(compiled_leaves::ActiveModel {
                    cohort_address: Set(cohort_metadata.cohort_address.to_string()),
                    claimant: Set(claimant.to_string()),
                    entitlements: Set(entitlements.to_string()),
                    vault_index: Set(claim_leaf.vault_index.into()),
                });

                proofs.push(compiled_proofs::ActiveModel {
                    cohort_address: Set(cohort_metadata.cohort_address.to_string()),
                    claimant: Set(claimant.to_string()),
                    merkle_proof: Set(merkle_proof),
                });
            }

            for vault_index in 0..cohort_metadata.vault_count {
                let vault_entitlements = vault_total_entitlements
                    .get(&vault_index)
                    .cloned()
                    .unwrap_or_default();

                // Calculate vault budget using budget allocator
                let VaultAllocation {
                    vault_budget_human,
                    vault_budget_tokens: vault_budget_token,
                    amount_per_entitlement_human,
                    amount_per_entitlement_tokens: amount_per_entitlement_token,
                    dust_amount_human,
                    dust_amount_tokens: dust_amount_token,
                } = budget_allocator.calculate_vault_allocation(
                    cohort_metadata.cohort_budget_human,
                    vault_entitlements,
                    cohort_metadata.total_entitlements,
                )?;

                vaults.push(
                    build_compiled_vault(
                        &address_finder,
                        &cohort_metadata,
                        vault_index,
                        vault_budget_human,
                        vault_budget_token,
                        dust_amount_human,
                        dust_amount_token,
                        amount_per_entitlement_human,
                        amount_per_entitlement_token,
                        vault_entitlements,
                    )
                    .await?,
                );
            }

            cohorts.push(compiled_cohort);
        }

        compiled_cohorts::Entity::insert_many(cohorts)
            .exec(db)
            .await?;

        compiled_leaves::Entity::insert_many(leaves)
            .exec(db)
            .await?;

        compiled_proofs::Entity::insert_many(proofs)
            .exec(db)
            .await?;

        compiled_vaults::Entity::insert_many(vaults)
            .exec(db)
            .await?;
    }

    // 🔍 Debug assertion: Verify data was actually inserted
    debug_assert!(
        compiled_cohorts::Entity::find().count(db).await? > 0,
        "No cohorts found after insertion"
    );
    debug_assert!(
        compiled_leaves::Entity::find().count(db).await? > 0,
        "No leaves found after insertion"
    );
    debug_assert!(
        compiled_proofs::Entity::find().count(db).await? > 0,
        "No proofs found after insertion"
    );
    debug_assert!(
        compiled_vaults::Entity::find().count(db).await? > 0,
        "No vaults found after insertion"
    );

    Ok(())
}

struct CohortMetadata {
    cohort_address: Pubkey,
    cohort_budget_human: Decimal,
    total_entitlements: Decimal,
    claimant_entitlements: Vec<(Pubkey, u64)>,
    claim_tree: ClaimTree,
    vault_count: u8,
}

async fn build_compiled_cohort_and_metadata(
    address_finder: &AddressFinder,
    db: &DatabaseConnection,
    cohort_csv_row: &cohorts_csv_rows::Model,
    budget_allocator: &BudgetAllocator,
    claimants_per_vault: Decimal,
    claim_tree_type: &ClaimTreeType,
) -> CompilerResult<(compiled_cohorts::ActiveModel, CohortMetadata)> {
    let claimant_entitlements: Vec<(Pubkey, u64)> = campaign_csv_rows::Entity::find()
        .filter(campaign_csv_rows::Column::Cohort.eq(&cohort_csv_row.cohort))
        .all(db)
        .await?
        .iter()
        .map(|row| Ok((row.claimant.parse()?, row.entitlements.try_into()?)))
        .collect::<Result<_, CompilerError>>()?;

    let claimant_count = Decimal::from(claimant_entitlements.len());

    let total_entitlements = claimant_entitlements
        .iter()
        .map(|(_, entitlements)| Decimal::from(*entitlements))
        .sum::<Decimal>();

    let vault_count = (claimant_count / claimants_per_vault).ceil();

    let vault_count: u8 =
        vault_count
            .try_into()
            .map_err(|_| CompilerError::VaultLimitExceeded {
                claimant_count,
                claimants_per_vault,
                vault_count,
            })?;

    let claim_tree = claim_tree_type.new_tree(
        address_finder.campaign, //
        &claimant_entitlements,
        vault_count,
    )?;

    let Some(merkle_root) = claim_tree.root() else {
        return Err(CompilerError::MerkleRootIsRequired);
    };

    let (cohort_address, _) = address_finder.find_cohort_v0_address(&merkle_root);

    let CohortAllocation {
        cohort_budget_human,
        cohort_budget_tokens: cohort_budget_token,
        amount_per_entitlement_human,
        amount_per_entitlement_tokens: amount_per_entitlement_token,
        dust_amount_human,
        dust_amount_tokens: dust_amount_token,
    } = budget_allocator.calculate_cohort_allocation(
        cohort_csv_row.share_percentage.parse()?,
        total_entitlements,
    )?;

    let cohort = compiled_cohorts::ActiveModel {
        address: Set(cohort_address.to_string()),
        cohort_csv_row_id: Set(cohort_csv_row.id),
        merkle_root: Set(hex::encode(merkle_root)),
        vault_count: Set(vault_count.to_string()),
        total_entitlements: Set(total_entitlements.to_string()),
        cohort_budget_human: Set(cohort_budget_human.to_string()),
        cohort_budget_token: Set(cohort_budget_token.to_string()),
        amount_per_entitlement_human: Set(amount_per_entitlement_human.to_string()),
        amount_per_entitlement_token: Set(amount_per_entitlement_token.to_string()),
        dust_amount_human: Set(dust_amount_human.to_string()),
        dust_amount_token: Set(dust_amount_token.to_string()),
    };

    let metadata = CohortMetadata {
        cohort_address,
        cohort_budget_human,
        total_entitlements,
        claimant_entitlements,
        claim_tree,
        vault_count,
    };

    Ok((cohort, metadata))
}

async fn build_compiled_vault(
    address_finder: &AddressFinder,
    cohort_metadata: &CohortMetadata,
    vault_index: u8,
    vault_budget_human: Decimal,
    vault_budget_token: u64,
    vault_dust_human: Decimal,
    vault_dust_token: u64,
    amount_per_entitlement_human: Decimal,
    amount_per_entitlement_token: u64,
    total_entitlements: Decimal,
) -> CompilerResult<compiled_vaults::ActiveModel> {
    // Allow zero entitlements due to consistent hashing collisions
    debug_assert!(total_entitlements >= Decimal::ZERO);

    let (vault_address, _) = address_finder.find_vault_v0_address(
        &cohort_metadata.cohort_address, //
        vault_index,
    );

    Ok(compiled_vaults::ActiveModel {
        vault_address: Set(vault_address.to_string()),
        cohort_address: Set(cohort_metadata.cohort_address.to_string()),
        vault_index: Set(vault_index.into()),
        vault_budget_human: Set(vault_budget_human.to_string()),
        vault_budget_token: Set(vault_budget_token.to_string()),
        vault_dust_human: Set(vault_dust_human.to_string()),
        vault_dust_token: Set(vault_dust_token.to_string()),
        amount_per_entitlement_human: Set(amount_per_entitlement_human.to_string()),
        amount_per_entitlement_token: Set(amount_per_entitlement_token.to_string()),
        total_entitlements: Set(total_entitlements.to_string()),
    })
}

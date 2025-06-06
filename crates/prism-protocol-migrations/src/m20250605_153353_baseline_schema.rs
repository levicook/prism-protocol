use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CampaignCsvRows::Table)
                    .if_not_exists()
                    .col(integer(CampaignCsvRows::Id).primary_key())
                    .col(string(CampaignCsvRows::Cohort))
                    .col(string(CampaignCsvRows::Claimant))
                    .col(integer(CampaignCsvRows::Entitlements))
                    .index(
                        Index::create()
                            .col(CampaignCsvRows::Cohort)
                            .col(CampaignCsvRows::Claimant)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CohortsCsvRows::Table)
                    .if_not_exists()
                    .col(integer(CohortsCsvRows::Id).primary_key())
                    .col(string(CohortsCsvRows::Cohort))
                    .col(string(CohortsCsvRows::SharePercentage))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CompiledCampaigns::Table)
                    .if_not_exists()
                    .col(string(CompiledCampaigns::Address).primary_key())
                    .col(string(CompiledCampaigns::CampaignAdmin))
                    .col(string(CompiledCampaigns::CampaignMint))
                    .col(string(CompiledCampaigns::CampaignBudgetHuman)) // Decimal
                    .col(string(CompiledCampaigns::CampaignBudgetToken)) // u64
                    .col(small_unsigned(CompiledCampaigns::MintDecimals))
                    .col(unsigned(CompiledCampaigns::ClaimantsPerVault))
                    .col(string(CompiledCampaigns::ClaimTreeType))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CompiledCohorts::Table)
                    .if_not_exists()
                    .col(string(CompiledCohorts::Address).primary_key())
                    .col(integer(CompiledCohorts::CohortCsvRowId))
                    .col(string(CompiledCohorts::MerkleRoot))
                    .col(string(CompiledCohorts::VaultCount))
                    .col(string(CompiledCohorts::TotalEntitlements))
                    .col(string(CompiledCohorts::CohortBudgetHuman)) // Decimal
                    .col(string(CompiledCohorts::CohortBudgetToken)) // u64
                    .col(string(CompiledCohorts::AmountPerEntitlementHuman)) // Decimal
                    .col(string(CompiledCohorts::AmountPerEntitlementToken)) // u64
                    .col(string(CompiledCohorts::DustAmountHuman)) // Decimal
                    .col(string(CompiledCohorts::DustAmountToken)) // u64
                    .foreign_key(
                        ForeignKey::create()
                            .from(CompiledCohorts::Table, CompiledCohorts::CohortCsvRowId)
                            .to(CohortsCsvRows::Table, CohortsCsvRows::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CompiledVaults::Table)
                    .if_not_exists()
                    .col(string(CompiledVaults::VaultAddress).primary_key())
                    .col(string(CompiledVaults::CohortAddress))
                    .col(small_unsigned(CompiledVaults::VaultIndex))
                    .col(string(CompiledVaults::VaultBudgetHuman)) // Decimal
                    .col(string(CompiledVaults::VaultBudgetToken)) // u64
                    .col(string(CompiledVaults::VaultDustHuman)) // Decimal
                    .col(string(CompiledVaults::VaultDustToken)) // u64
                    .col(string(CompiledVaults::AmountPerEntitlementHuman)) // Decimal
                    .col(string(CompiledVaults::AmountPerEntitlementToken)) // u64
                    .col(string(CompiledVaults::TotalEntitlements))
                    .foreign_key(
                        ForeignKey::create()
                            .from(CompiledVaults::Table, CompiledVaults::CohortAddress)
                            .to(CompiledCohorts::Table, CompiledCohorts::Address)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CompiledLeaves::Table)
                    .if_not_exists()
                    .col(string(CompiledLeaves::CohortAddress))
                    .col(string(CompiledLeaves::Claimant))
                    .col(string(CompiledLeaves::Entitlements))
                    .col(small_unsigned(CompiledLeaves::VaultIndex))
                    .index(
                        Index::create()
                            .col(CompiledLeaves::CohortAddress)
                            .col(CompiledLeaves::Claimant)
                            .primary(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(CompiledLeaves::Table, CompiledLeaves::CohortAddress)
                            .to(CompiledCohorts::Table, CompiledCohorts::Address)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CompiledProofs::Table)
                    .if_not_exists()
                    .col(string(CompiledProofs::CohortAddress))
                    .col(string(CompiledProofs::Claimant))
                    .col(string(CompiledProofs::MerkleProof))
                    .index(
                        Index::create()
                            .col(CompiledProofs::CohortAddress)
                            .col(CompiledProofs::Claimant)
                            .primary(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(CompiledProofs::Table, CompiledProofs::CohortAddress)
                            .to(CompiledCohorts::Table, CompiledCohorts::Address)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CompiledProofs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CompiledLeaves::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CompiledVaults::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CompiledCohorts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CompiledCampaigns::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CohortsCsvRows::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CampaignCsvRows::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum CampaignCsvRows {
    Table,
    Id,
    Cohort,
    Claimant,
    Entitlements,
}

#[derive(DeriveIden)]
enum CohortsCsvRows {
    Table,
    Id,
    Cohort,
    SharePercentage,
}

#[derive(DeriveIden)]
enum CompiledCampaigns {
    Table,
    Address,
    CampaignAdmin,
    CampaignMint,
    CampaignBudgetHuman, // campaign budget (Decimal, ie: SOL)
    CampaignBudgetToken, // campaign budget (u64, ie: lamports)
    MintDecimals,
    ClaimantsPerVault,
    ClaimTreeType,
}

#[derive(DeriveIden)]
enum CompiledCohorts {
    Table,
    Address,
    CohortCsvRowId,
    MerkleRoot,

    VaultCount, // number of vaults in the cohort (campaign.claimants_per_vault / count(claimants))

    TotalEntitlements, // sum of all entitlements in the cohort

    CohortBudgetHuman, // budget allocated to the cohort (Decimal, ie: SOL)
    CohortBudgetToken, // budget allocated to the cohort (u64, ie: lamports)

    AmountPerEntitlementHuman, // amount per entitlement (Decimal, ie: SOL)
    AmountPerEntitlementToken, // amount per entitlement (u64, ie: lamports)

    DustAmountHuman, // amount that couldn't be allocated due to mint constraints (Decimal, ie: SOL)
    DustAmountToken, // amount that couldn't be allocated due to mint constraints (u64, ie: lamports)
}

#[derive(DeriveIden)]
enum CompiledVaults {
    Table,
    CohortAddress, // foreign key to cohorts.address
    VaultAddress,  // pubkey of the vault
    VaultIndex,    // index of the vault

    VaultBudgetHuman, // budget allocated to the vault (Decimal, ie: SOL)
    VaultBudgetToken, // budget allocated to the vault (u64, ie: lamports)

    VaultDustHuman, // amount that couldn't be allocated due to mint constraints (Decimal, ie: SOL)
    VaultDustToken, // amount that couldn't be allocated due to mint constraints (u64, ie: lamports)

    AmountPerEntitlementHuman, // amount per entitlement (per the allocator) (Decimal, ie: SOL)
    AmountPerEntitlementToken, // amount per entitlement (per the allocator) (u64, ie: lamports)

    TotalEntitlements, // sum of all entitlements in the vault
}

#[derive(DeriveIden)]
enum CompiledLeaves {
    Table,
    CohortAddress, // foreign key to cohorts.address

    Claimant,     // -------\
    Entitlements, //         +---- Claim Leaf
    VaultIndex,   // -------/
}

#[derive(DeriveIden)]
enum CompiledProofs {
    Table,
    CohortAddress, // foreign key to cohorts.address
    Claimant,
    MerkleProof,
}

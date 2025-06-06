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
                    .table(Campaigns::Table)
                    .if_not_exists()
                    .col(string(Campaigns::Address).primary_key())
                    .col(string(Campaigns::CampaignAdmin))
                    .col(string(Campaigns::CampaignMint))
                    .col(string(Campaigns::CampaignBudget))
                    .col(small_unsigned(Campaigns::MintDecimals))
                    .col(unsigned(Campaigns::ClaimantsPerVault))
                    .col(string(Campaigns::ClaimTreeType))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Cohorts::Table)
                    .if_not_exists()
                    .col(string(Cohorts::Address).primary_key())
                    .col(integer(Cohorts::CohortCsvRowId))
                    .col(string(Cohorts::MerkleRoot))
                    .col(string(Cohorts::VaultCount))
                    .col(string(Cohorts::TotalEntitlements))
                    .col(string(Cohorts::CohortBudget))
                    .col(string(Cohorts::AmountPerEntitlement))
                    .col(string(Cohorts::DustAmount))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Vaults::Table)
                    .if_not_exists()
                    .col(string(Vaults::VaultAddress).primary_key())
                    .col(string(Vaults::CohortAddress))
                    .col(string(Vaults::VaultIndex))
                    .col(string(Vaults::VaultBudget))
                    .col(string(Vaults::VaultDust))
                    .col(string(Vaults::AmountPerEntitlement))
                    .col(string(Vaults::TotalEntitlements))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ClaimLeaves::Table)
                    .if_not_exists()
                    .col(string(ClaimLeaves::CohortAddress))
                    .col(string(ClaimLeaves::Claimant))
                    .col(string(ClaimLeaves::Entitlements))
                    .col(string(ClaimLeaves::VaultIndex))
                    .index(
                        Index::create()
                            .col(ClaimLeaves::CohortAddress)
                            .col(ClaimLeaves::Claimant)
                            .primary(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ClaimProofs::Table)
                    .if_not_exists()
                    .col(string(ClaimProofs::CohortAddress))
                    .col(string(ClaimProofs::Claimant))
                    .col(string(ClaimProofs::MerkleProof))
                    .index(
                        Index::create()
                            .col(ClaimProofs::CohortAddress)
                            .col(ClaimProofs::Claimant)
                            .primary(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ClaimProofs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ClaimLeaves::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Vaults::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Cohorts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Campaigns::Table).to_owned())
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
enum Campaigns {
    Table,
    Address,
    CampaignAdmin,
    CampaignMint,
    CampaignBudget,
    MintDecimals,
    ClaimantsPerVault,
    ClaimTreeType,
}

#[derive(DeriveIden)]
enum Cohorts {
    Table,
    Address,
    CohortCsvRowId,
    MerkleRoot,

    VaultCount, // number of vaults in the cohort (campaign.claimants_per_vault / count(claimants))

    TotalEntitlements,    // sum of all entitlements in the cohort
    CohortBudget,         // budget allocated to the cohort
    AmountPerEntitlement, // amount per entitlement
    DustAmount,           // amount that couldn't be allocated due to mint constraints
}

#[derive(DeriveIden)]
enum Vaults {
    Table,
    CohortAddress,        // foreign key to cohorts.address
    VaultAddress,         // pubkey of the vault
    VaultIndex,           // index of the vault
    VaultBudget,          // budget allocated to the vault
    VaultDust,            // amount that couldn't be allocated due to mint constraints
    AmountPerEntitlement, // amount per entitlement (per the allocator)
    TotalEntitlements,    // sum of all entitlements in the vault
}

#[derive(DeriveIden)]
enum ClaimLeaves {
    Table,

    CohortAddress, // foreign key to cohorts.address

    // Merkle Leaf:
    Claimant,
    Entitlements,
    VaultIndex,
}

#[derive(DeriveIden)]
enum ClaimProofs {
    Table,

    CohortAddress, // foreign key to cohorts.address

    Claimant,
    MerkleProof,
}

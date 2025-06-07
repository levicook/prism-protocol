mod compiled_campaign_ext;
mod compiled_cohort_ext;
mod compiled_leaf_ext;
mod compiled_proof_ext;
mod compiled_vault_ext;

use anchor_spl::token::spl_token;
use prism_protocol_entities::{
    compiled_campaigns, compiled_cohorts, compiled_leaves, compiled_proofs, compiled_vaults,
};
use sea_orm::{
    ColumnTrait as _, DatabaseConnection, EntityTrait as _, PaginatorTrait as _, QueryFilter as _,
    QueryOrder,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair};

use crate::{
    build_activate_campaign_v0_ix, build_activate_cohort_v0_ix, build_activate_vault_v0_ix,
    build_claim_tokens_v0_ix, build_claim_tokens_v1_ix, build_initialize_campaign_v0_ix,
    build_initialize_cohort_v0_ix, build_initialize_vault_v0_ix,
    build_make_campaign_unstoppable_v0_ix, build_pause_campaign_v0_ix,
    build_permanently_halt_campaign_v0_ix, build_reclaim_tokens_v0_ix, build_resume_campaign_v0_ix,
    AddressFinder, ClaimTreeType,
};

pub use {
    compiled_campaign_ext::CompiledCampaignExt, compiled_cohort_ext::CompiledCohortExt,
    compiled_leaf_ext::CompiledLeafExt, compiled_proof_ext::CompiledProofExt,
    compiled_vault_ext::CompiledVaultExt,
};

pub struct CompiledCampaignDatabase {
    pub address_finder: AddressFinder,
    pub campaign_keypair: Option<Keypair>,
    db: DatabaseConnection,
}

impl CompiledCampaignDatabase {
    pub fn new(address_finder: AddressFinder, db: DatabaseConnection) -> Self {
        Self {
            address_finder,
            campaign_keypair: None,
            db,
        }
    }

    pub fn new_with_keypair(
        address_finder: AddressFinder,
        db: DatabaseConnection,
        campaign_keypair: Keypair,
    ) -> Self {
        Self {
            address_finder,
            campaign_keypair: Some(campaign_keypair),
            db,
        }
    }

    pub async fn build_initialize_campaign_ix(&self) -> anchor_lang::Result<Instruction> {
        let expected_cohort_count = self.compiled_cohort_count().await;
        let (ix, _, _) = build_initialize_campaign_v0_ix(
            &self.address_finder, //
            expected_cohort_count,
        )?;
        Ok(ix)
    }

    pub async fn build_initialize_cohort_ixs(&self) -> anchor_lang::Result<Vec<Instruction>> {
        self.compiled_cohorts()
            .await
            .into_iter()
            .map(|cohort| {
                let (ix, _, _) = build_initialize_cohort_v0_ix(
                    &self.address_finder,
                    cohort.merkle_root(),
                    cohort.amount_per_entitlement_token(),
                    cohort.vault_count(),
                )?;
                Ok(ix)
            })
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn build_initialize_vault_ixs(&self) -> anchor_lang::Result<Vec<Instruction>> {
        let mut ix_vec = Vec::new();

        for cohort in self.compiled_cohorts().await {
            let vaults = self
                .compiled_vaults_by_cohort_address(cohort.address())
                .await;

            for vault in vaults {
                let (ix, _, _) = build_initialize_vault_v0_ix(
                    &self.address_finder,
                    cohort.merkle_root(),
                    vault.vault_index(),
                )?;
                ix_vec.push(ix);
            }
        }

        Ok(ix_vec)
    }

    pub async fn build_fund_vault_ixs(&self) -> anchor_lang::Result<Vec<Instruction>> {
        let mut ix_vec = Vec::new();

        // Use AddressFinder to get admin's associated token account
        let admin_token_account = self.address_finder.find_admin_token_account();

        for vault in self.compiled_vaults().await {
            // Fund vault with budget minus dust (dust can never be claimed due to rounding)
            // This ensures perfect vault drainage when all entitlements are claimed
            let claimable_amount = vault.vault_budget_token() - vault.vault_dust_token();

            ix_vec.push(spl_token::instruction::transfer(
                &self.address_finder.token_program_id,
                &admin_token_account,          // from: admin's ATA
                &vault.vault_address(),        // to: vault token account
                &self.address_finder.admin,    // authority: admin signs
                &[&self.address_finder.admin], // signers
                claimable_amount,              // amount (excluding mathematical dust)
            )?);
        }
        Ok(ix_vec)
    }

    pub async fn build_activate_vault_ixs(&self) -> anchor_lang::Result<Vec<Instruction>> {
        let mut ix_vec = Vec::new();
        for cohort in self.compiled_cohorts().await {
            let vaults = self
                .compiled_vaults_by_cohort_address(cohort.address())
                .await;
            for vault in vaults {
                // Expect vault balance to match funded amount (budget minus dust)
                let expected_balance = vault.vault_budget_token() - vault.vault_dust_token();

                let (ix, _, _) = build_activate_vault_v0_ix(
                    &self.address_finder,
                    cohort.merkle_root(),
                    vault.vault_index(),
                    expected_balance,
                )?;
                ix_vec.push(ix);
            }
        }
        Ok(ix_vec)
    }

    pub async fn build_activate_cohort_ixs(&self) -> anchor_lang::Result<Vec<Instruction>> {
        self.compiled_cohorts()
            .await
            .into_iter()
            .map(|cohort| {
                let (ix, _, _) = build_activate_cohort_v0_ix(
                    &self.address_finder, //
                    cohort.merkle_root(),
                )?;
                Ok(ix)
            })
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn build_activate_campaign_ix(
        &self,
        final_db_ipfs_hash: [u8; 32],
        go_live_slot: u64,
    ) -> anchor_lang::Result<Instruction> {
        let (ix, _, _) = build_activate_campaign_v0_ix(
            &self.address_finder, //
            final_db_ipfs_hash,
            go_live_slot,
        )?;
        Ok(ix)
    }

    pub async fn build_make_campaign_unstoppable_ix(&self) -> anchor_lang::Result<Instruction> {
        let (ix, _, _) = build_make_campaign_unstoppable_v0_ix(&self.address_finder)?;
        Ok(ix)
    }

    pub async fn build_pause_campaign_ix(&self) -> anchor_lang::Result<Instruction> {
        let (ix, _, _) = build_pause_campaign_v0_ix(&self.address_finder)?;
        Ok(ix)
    }

    pub async fn build_resume_campaign_ix(&self) -> anchor_lang::Result<Instruction> {
        let (ix, _, _) = build_resume_campaign_v0_ix(&self.address_finder)?;
        Ok(ix)
    }

    pub async fn build_permanently_halt_campaign_ix(&self) -> anchor_lang::Result<Instruction> {
        let (ix, _, _) = build_permanently_halt_campaign_v0_ix(&self.address_finder)?;
        Ok(ix)
    }

    pub async fn build_claim_tokens_ixs(
        &self,
        claimant: Pubkey,
    ) -> anchor_lang::Result<Vec<Instruction>> {
        let campaign = self.compiled_campaign().await;
        let claim_tree_type = campaign.claim_tree_type();

        let proofs = self.compiled_proofs_by_claimant(claimant).await;

        let mut ix_vec = Vec::new();

        for proof in proofs {
            let cohort = self
                .compiled_cohort_by_address(proof.cohort_address())
                .await;

            let leaf = self
                .compiled_leaf_by_cohort_and_claimant(cohort.address(), claimant)
                .await;

            ix_vec.push(match claim_tree_type {
                ClaimTreeType::V0 => {
                    let (ix, _, _) = build_claim_tokens_v0_ix(
                        &self.address_finder,
                        claimant,
                        cohort.merkle_root(),
                        proof.merkle_proof_v0(),
                        leaf.vault_index(),
                        leaf.entitlements(),
                    )?;

                    ix
                }
                ClaimTreeType::V1 => {
                    let (ix, _, _) = build_claim_tokens_v1_ix(
                        &self.address_finder,
                        claimant,
                        cohort.merkle_root(),
                        proof.merkle_proof_v1(),
                        leaf.vault_index(),
                        leaf.entitlements(),
                    )?;
                    ix
                }
            });
        }

        Ok(ix_vec)
    }

    pub async fn build_reclaim_tokens_ixs(&self) -> anchor_lang::Result<Vec<Instruction>> {
        let mut ix_vec = Vec::new();
        for cohort in self.compiled_cohorts().await {
            let vaults = self
                .compiled_vaults_by_cohort_address(cohort.address())
                .await;
            for vault in vaults {
                let (ix, _, _) = build_reclaim_tokens_v0_ix(
                    &self.address_finder,
                    cohort.merkle_root(),
                    vault.vault_index(),
                )?;
                ix_vec.push(ix);
            }
        }
        Ok(ix_vec)
    }

    pub async fn compiled_campaign_address(&self) -> Pubkey {
        let compiled_campaign = self.compiled_campaign().await;
        compiled_campaign.address()
    }

    pub async fn compiled_campaign(&self) -> compiled_campaigns::Model {
        let campaign_address = self.address_finder.campaign.to_string();
        compiled_campaigns::Entity::find()
            .filter(compiled_campaigns::Column::Address.eq(campaign_address))
            .one(&self.db)
            .await
            .expect("Failed to fetch campaign account")
            .expect("Campaign not found")
    }

    pub async fn compiled_cohort_addresses(&self) -> Vec<Pubkey> {
        self.compiled_cohorts()
            .await
            .into_iter()
            .map(|c| c.address())
            .collect()
    }

    pub async fn compiled_cohort_count(&self) -> u8 {
        compiled_cohorts::Entity::find()
            .count(&self.db)
            .await
            .expect("Failed to fetch cohorts count")
            .try_into()
            .expect("Cohorts count too large")
    }

    pub async fn compiled_cohorts(&self) -> Vec<compiled_cohorts::Model> {
        compiled_cohorts::Entity::find()
            .order_by_asc(compiled_cohorts::Column::Address)
            .all(&self.db)
            .await
            .expect("Failed to fetch cohorts")
            .into_iter()
            .map(|c| c.into())
            .collect()
    }

    pub async fn compiled_cohort_by_address(&self, address: Pubkey) -> compiled_cohorts::Model {
        compiled_cohorts::Entity::find()
            .filter(compiled_cohorts::Column::Address.eq(address.to_string()))
            .one(&self.db)
            .await
            .expect("Failed to fetch cohort")
            .expect("Cohort not found")
    }

    pub async fn compiled_leaf_by_cohort_and_claimant(
        &self,
        cohort_address: Pubkey,
        claimant: Pubkey,
    ) -> compiled_leaves::Model {
        compiled_leaves::Entity::find()
            .filter(compiled_leaves::Column::CohortAddress.eq(cohort_address.to_string()))
            .filter(compiled_leaves::Column::Claimant.eq(claimant.to_string()))
            .one(&self.db)
            .await
            .expect("Failed to fetch leaf")
            .expect("Leaf not found")
    }

    pub async fn compiled_proofs_by_claimant(
        &self,
        claimant: Pubkey,
    ) -> Vec<compiled_proofs::Model> {
        compiled_proofs::Entity::find()
            .order_by_asc(compiled_proofs::Column::CohortAddress)
            .filter(compiled_proofs::Column::Claimant.eq(claimant.to_string()))
            .all(&self.db)
            .await
            .expect("Failed to fetch proofs")
    }

    pub async fn compiled_vault_addresses(&self) -> Vec<Pubkey> {
        self.compiled_vaults()
            .await
            .into_iter()
            .map(|v| v.vault_address())
            .collect()
    }

    pub async fn compiled_vaults(&self) -> Vec<compiled_vaults::Model> {
        compiled_vaults::Entity::find()
            .order_by_asc(compiled_vaults::Column::CohortAddress)
            .order_by_asc(compiled_vaults::Column::VaultIndex)
            .all(&self.db)
            .await
            .expect("Failed to fetch vaults")
            .into_iter()
            .map(|v| v.into())
            .collect()
    }

    pub async fn compiled_vaults_by_cohort_address(
        &self,
        cohort_address: Pubkey,
    ) -> Vec<compiled_vaults::Model> {
        compiled_vaults::Entity::find()
            .filter(compiled_vaults::Column::CohortAddress.eq(cohort_address.to_string()))
            .order_by_asc(compiled_vaults::Column::VaultIndex)
            .all(&self.db)
            .await
            .expect("Failed to fetch vaults")
    }

    pub async fn compiled_vault_by_address(
        &self,
        address: Pubkey,
    ) -> Option<compiled_vaults::Model> {
        compiled_vaults::Entity::find()
            .filter(compiled_vaults::Column::VaultAddress.eq(address.to_string()))
            .one(&self.db)
            .await
            .expect("Failed to fetch vault")
    }
}

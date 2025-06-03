use {
    crate::{create_mint, load_prism_protocol, FixtureStage, FixtureState},
    litesvm::{
        types::{FailedTransactionMetadata, TransactionResult},
        LiteSVM,
    },
    litesvm_token::spl_token::solana_program::native_token::LAMPORTS_PER_SOL,
    prism_protocol_sdk::{
        build_activate_campaign_v0_ix, build_activate_cohort_v0_ix, build_activate_vault_v0_ix,
        build_initialize_campaign_v0_ix, build_initialize_cohort_v0_ix,
        build_initialize_vault_v0_ix,
    },
    rust_decimal::prelude::ToPrimitive,
    solana_instruction::Instruction,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer as _,
    solana_sysvar::clock::Clock,
    solana_transaction::Transaction,
};

pub struct TestFixture {
    pub state: FixtureState,

    log_send_transaction_results: bool,
    svm: LiteSVM,
}

impl TestFixture {
    pub fn new(state: FixtureState, mut svm: LiteSVM) -> Result<Self, FailedTransactionMetadata> {
        load_prism_protocol(&mut svm, state.address_finder.program_id);

        svm.airdrop(&state.compiled_campaign.admin, LAMPORTS_PER_SOL * 100)?;

        create_mint(
            &mut svm,
            &state.admin_keypair,
            &state.mint_keypair,
            state.compiled_campaign.mint_decimals,
            None,
        )?;

        Ok(Self {
            state,
            svm,
            log_send_transaction_results: true,
        })
    }

    pub fn airdrop(&mut self, to: &Pubkey, amount: u64) -> TransactionResult {
        self.svm.airdrop(to, amount)
    }

    pub fn latest_blockhash(&self) -> solana_hash::Hash {
        self.svm.latest_blockhash()
    }

    pub fn current_slot(&self) -> u64 {
        self.svm.get_sysvar::<Clock>().slot
    }

    pub fn warp_to_slot(&mut self, slot: u64) {
        self.svm.warp_to_slot(slot);
    }

    pub fn disable_send_transaction_logging(&mut self) {
        self.log_send_transaction_results = false;
    }

    pub fn enable_send_transaction_logging(&mut self) {
        self.log_send_transaction_results = true;
    }

    pub fn send_instructions(&mut self, instructions: &[Instruction]) -> TransactionResult {
        let fee_payer = &self.state.admin_keypair;

        let tx = Transaction::new(
            &[fee_payer],
            Message::new(instructions, Some(&fee_payer.pubkey())),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)
    }

    /// Send a transaction and optionally print logs based on the logging_enabled setting
    pub fn send_transaction(&mut self, tx: Transaction) -> TransactionResult {
        let result = self.svm.send_transaction(tx);

        if self.log_send_transaction_results {
            match &result {
                Ok(meta) => {
                    println!("=== Transaction Logs (Success) ===");
                    for (i, log) in meta.logs.iter().enumerate() {
                        println!("{}: {}", i, log);
                    }
                    println!("=== End Logs ===\n");
                }
                Err(failed_meta) => {
                    println!("=== Transaction Logs (Failed) ===");
                    for (i, log) in failed_meta.meta.logs.iter().enumerate() {
                        println!("{}: {}", i, log);
                    }
                    println!("Error: {:?}", failed_meta.err);
                    println!("=== End Logs ===\n");
                }
            }
        }

        result
    }

    pub fn send_transactions(
        &mut self,
        txs: Vec<Transaction>,
    ) -> Result<(), FailedTransactionMetadata> {
        for tx in txs {
            self.send_transaction(tx)?;
        }
        Ok(())
    }

    pub fn jump_to(&mut self, target_stage: FixtureStage) -> Result<(), FailedTransactionMetadata> {
        // step all stages:
        // - greater than campaign compiled (initial state)
        // - greater than current stage
        // - less than or equal to target stage

        let current_stage = &self.state.stage;
        let mut stages_to_step = FixtureStage::all()
            .to_vec()
            .into_iter()
            .filter(|s| {
                s > &FixtureStage::CampaignCompiled && s > current_stage && s <= &target_stage
            })
            .collect::<Vec<_>>();

        // ensure they are in order (unlikely to be out of order, better safe than sorry)
        stages_to_step.sort_by(|a, b| a.cmp(b));

        for stage in stages_to_step {
            self.step_to(stage)?;
        }

        Ok(())
    }

    pub fn step_to(&mut self, stage: FixtureStage) -> Result<(), FailedTransactionMetadata> {
        match stage {
            FixtureStage::CampaignCompiled => {
                // nothing to do (intentional no-op)
            }
            FixtureStage::CampaignInitialized => {
                self.advance_to_campaign_initialized()?;
                self.state.stage = FixtureStage::CampaignInitialized;
            }
            FixtureStage::CohortsInitialized => {
                self.advance_to_cohorts_initialized()?;
                self.state.stage = FixtureStage::CohortsInitialized;
            }
            FixtureStage::VaultsInitialized => {
                self.advance_to_vaults_initialized()?;
                self.state.stage = FixtureStage::VaultsInitialized;
            }
            FixtureStage::VaultsActivated => {
                self.advance_to_vaults_activated()?;
                self.state.stage = FixtureStage::VaultsActivated;
            }
            FixtureStage::CohortsActivated => {
                self.advance_to_cohorts_activated()?;
                self.state.stage = FixtureStage::CohortsActivated;
            }
            FixtureStage::CampaignsActivated => {
                self.advance_to_campaign_activated()?;
                self.state.stage = FixtureStage::CampaignsActivated;
            }
        }
        Ok(())
    }

    fn advance_to_campaign_initialized(&mut self) -> Result<(), FailedTransactionMetadata> {
        let (ix, _, _) = build_initialize_campaign_v0_ix(
            &self.state.address_finder,
            self.state.compiled_campaign.admin,
            self.state.compiled_campaign.fingerprint,
            self.state.compiled_campaign.mint,
            self.state
                .compiled_campaign
                .cohorts
                .len()
                .try_into()
                .expect("Cohort count too large"),
        )
        .expect("Failed to build initialize campaign v0 ix");

        let tx = Transaction::new(
            &[&self.state.admin_keypair],
            Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        Ok(())
    }

    fn advance_to_cohorts_initialized(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        for cohort in &self.state.compiled_campaign.cohorts {
            let amount_per_entitlement = cohort
                .amount_per_entitlement
                .floor()
                .to_u64()
                .expect("Amount too large");

            let expected_vault_count = cohort
                .vault_count
                .try_into()
                .expect("Vault count too large");

            let (ix, _, _) = build_initialize_cohort_v0_ix(
                &self.state.address_finder,
                self.state.compiled_campaign.admin,
                self.state.compiled_campaign.fingerprint,
                cohort.merkle_root,
                amount_per_entitlement,
                expected_vault_count,
            )
            .expect("Failed to build initialize cohort v0 ix");

            txs.push(Transaction::new(
                &[&self.state.admin_keypair],
                Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
                self.latest_blockhash(),
            ));
        }

        self.send_transactions(txs)
    }

    fn advance_to_vaults_initialized(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        for cohort in &self.state.compiled_campaign.cohorts {
            for (vault_index, _) in cohort.vaults.iter().enumerate() {
                let (ix, _, _) = build_initialize_vault_v0_ix(
                    &self.state.address_finder,
                    self.state.compiled_campaign.admin,
                    self.state.compiled_campaign.fingerprint,
                    cohort.merkle_root,
                    self.state.compiled_campaign.mint,
                    vault_index.try_into().expect("Vault index too large"),
                )
                .expect("Failed to build initialize vault v0 ix");

                txs.push(Transaction::new(
                    &[&self.state.admin_keypair],
                    Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
                    self.latest_blockhash(),
                ));
            }
        }

        self.send_transactions(txs)
    }

    fn advance_to_vaults_activated(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        for cohort in &self.state.compiled_campaign.cohorts {
            for (vault_index, vault) in cohort.vaults.iter().enumerate() {
                let expected_balance = vault
                    .required_tokens_u64()
                    .expect("Required tokens too large");

                let ix1 = spl_token::instruction::mint_to(
                    &self.state.address_finder.token_program_id,
                    &self.state.compiled_campaign.mint,
                    &vault.address,
                    &self.state.admin_keypair.pubkey(),
                    &[&self.state.admin_keypair.pubkey()],
                    expected_balance,
                )
                .expect("Failed to build mint_to ix");

                let (ix2, _, _) = build_activate_vault_v0_ix(
                    &self.state.address_finder,
                    self.state.compiled_campaign.admin,
                    self.state.compiled_campaign.fingerprint,
                    cohort.merkle_root,
                    vault_index.try_into().expect("Vault index too large"),
                    expected_balance,
                )
                .expect("Failed to build activate vault v0 ix");

                txs.push(Transaction::new(
                    &[&self.state.admin_keypair],
                    Message::new(&[ix1, ix2], Some(&self.state.compiled_campaign.admin)),
                    self.latest_blockhash(),
                ));
            }
        }

        self.send_transactions(txs)
    }

    fn advance_to_cohorts_activated(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        for cohort in &self.state.compiled_campaign.cohorts {
            let (ix, _, _) = build_activate_cohort_v0_ix(
                &self.state.address_finder,
                self.state.compiled_campaign.admin,
                self.state.compiled_campaign.fingerprint,
                cohort.merkle_root,
            )
            .expect("Failed to build activate cohort v0 ix");

            txs.push(Transaction::new(
                &[&self.state.admin_keypair],
                Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
                self.latest_blockhash(),
            ));
        }

        self.send_transactions(txs)
    }

    fn advance_to_campaign_activated(&mut self) -> Result<(), FailedTransactionMetadata> {
        // For test fixtures, we can use placeholder values
        let final_db_ipfs_hash = [1u8; 32]; // Placeholder IPFS hash
        let go_live_slot = self.current_slot() + 10; // Go live in 10 slots

        let (ix, _, _) = build_activate_campaign_v0_ix(
            &self.state.address_finder,
            self.state.compiled_campaign.admin,
            self.state.compiled_campaign.fingerprint,
            final_db_ipfs_hash,
            go_live_slot,
        )
        .expect("Failed to build activate campaign v0 ix");

        let tx = Transaction::new(
            &[&self.state.admin_keypair],
            Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        Ok(())
    }

    /// Get token account balance, returns 0 if account doesn't exist
    pub fn get_token_account_balance(&self, token_account: &Pubkey) -> Result<u64, &'static str> {
        match self.svm.get_account(token_account) {
            Some(account) => {
                // Token accounts are 165 bytes, check size
                if account.data.len() != 165 {
                    return Err("Invalid token account size");
                }
                
                // Token account amount is at bytes 64-72 (u64 little-endian)
                let amount_bytes: [u8; 8] = account.data[64..72]
                    .try_into()
                    .map_err(|_| "Failed to read amount bytes")?;
                
                Ok(u64::from_le_bytes(amount_bytes))
            }
            None => Ok(0), // Account doesn't exist = 0 balance
        }
    }

    /// Check if an account exists
    pub fn account_exists(&self, address: &Pubkey) -> bool {
        self.svm.get_account(address).is_some()
    }

    /// Advance the slot by a specific number
    pub fn advance_slot_by(&mut self, slots: u64) {
        let current_slot = self.current_slot();
        self.warp_to_slot(current_slot + slots);
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new(FixtureState::default(), LiteSVM::new())
            .expect("Failed to create default test fixture")
    }
}

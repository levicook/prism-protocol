use {
    crate::{create_mint, load_prism_protocol, FixtureStage, FixtureState},
    anchor_lang::AccountDeserialize,
    litesvm::{
        types::{FailedTransactionMetadata, TransactionResult},
        LiteSVM,
    },
    litesvm_token::spl_token::solana_program::native_token::LAMPORTS_PER_SOL,
    prism_protocol::{CampaignV0, ClaimLeaf},
    prism_protocol_sdk::{
        build_activate_campaign_v0_ix, build_activate_cohort_v0_ix, build_activate_vault_v0_ix,
        build_claim_tokens_v0_ix, build_initialize_campaign_v0_ix, build_initialize_cohort_v0_ix,
        build_initialize_vault_v0_ix, build_make_campaign_unstoppable_v0_ix,
        build_pause_campaign_v0_ix, build_permanently_halt_campaign_v0_ix,
        build_reclaim_tokens_v0_ix, build_resume_campaign_v0_ix, CompiledCohort,
    },
    rust_decimal::prelude::ToPrimitive,
    solana_instruction::Instruction,
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer as _,
    solana_sysvar::clock::Clock,
    solana_transaction::Transaction,
    spl_associated_token_account::get_associated_token_address,
    std::collections::HashMap,
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

    pub fn airdrop(&mut self, to: &Pubkey, amount: u64) {
        self.svm
            .airdrop(to, amount)
            .unwrap_or_else(|e| panic!("Failed to airdrop to {amount} {to}: {e:?}"));
    }

    pub fn latest_blockhash(&self) -> solana_hash::Hash {
        self.svm.latest_blockhash()
    }

    pub fn advance_slot_by(&mut self, slots: u64) {
        let current_slot = self.current_slot();
        self.warp_to_slot(current_slot + slots);
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

    pub fn jump_to(&mut self, target_stage: FixtureStage) {
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
            self.step_to(stage);
        }
    }

    pub fn step_to(&mut self, stage: FixtureStage) {
        match stage {
            FixtureStage::CampaignCompiled => return,
            FixtureStage::CampaignInitialized => self.try_initialize_campaign(),
            FixtureStage::CohortsInitialized => self.try_initialize_cohorts(),
            FixtureStage::VaultsInitialized => self.try_initialize_vaults(),
            FixtureStage::VaultsFunded => self.try_fund_vaults(),
            FixtureStage::VaultsActivated => self.try_activate_vaults(),
            FixtureStage::CohortsActivated => self.try_activate_cohorts(),
            FixtureStage::CampaignActivated => self.try_activate_campaign(),
        }
        .unwrap_or_else(|e| panic!("Failed to advance to {:?}: {:?}", stage, e));

        self.state.stage = stage;
    }

    pub fn try_initialize_campaign(&mut self) -> Result<(), FailedTransactionMetadata> {
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

    pub fn try_initialize_cohorts(&mut self) -> Result<(), FailedTransactionMetadata> {
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

    pub fn try_initialize_vaults(&mut self) -> Result<(), FailedTransactionMetadata> {
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

    pub fn try_fund_vaults(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        for cohort in &self.state.compiled_campaign.cohorts {
            for vault in cohort.vaults.iter() {
                let expected_balance = vault
                    .required_tokens_u64()
                    .expect("Required tokens too large");

                let ix = spl_token::instruction::mint_to(
                    &self.state.address_finder.token_program_id,
                    &self.state.compiled_campaign.mint,
                    &vault.address,
                    &self.state.admin_keypair.pubkey(),
                    &[&self.state.admin_keypair.pubkey()],
                    expected_balance,
                )
                .expect("Failed to build mint_to ix");

                txs.push(Transaction::new(
                    &[&self.state.admin_keypair],
                    Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
                    self.latest_blockhash(),
                ));
            }
        }

        self.send_transactions(txs)
    }

    /// Fund vaults with custom amounts, falling back to compiled campaign amounts for unspecified vaults
    ///
    /// This method allows selective override of vault funding amounts while maintaining
    /// the compiled campaign's calculated amounts for other vaults. This is essential
    /// for edge case testing where we need to create scenarios like:
    /// - Insufficient vault balance relative to expected claims
    /// - Boundary condition testing (exact amounts, off-by-one errors)
    /// - Vault funding mismatch scenarios
    ///
    /// # Arguments
    /// * `custom_amounts` - HashMap mapping vault addresses to custom funding amounts
    ///
    /// # Example
    /// ```rust
    /// use std::collections::HashMap;
    ///
    /// let custom_funding = HashMap::from([
    ///     (vault_address_1, 1000u64),  // Fund with only 1000 tokens
    ///     (vault_address_2, 0u64),     // Fund with 0 tokens (empty vault)
    /// ]);
    ///
    /// test.try_fund_vaults_with_custom_amounts(custom_funding)?;
    /// ```
    pub fn try_fund_vaults_with_custom_amounts(
        &mut self,
        custom_amounts: HashMap<Pubkey, u64>,
    ) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        for cohort in &self.state.compiled_campaign.cohorts {
            for vault in cohort.vaults.iter() {
                // Use custom amount if specified, otherwise fall back to compiled campaign amount
                let funding_amount =
                    custom_amounts
                        .get(&vault.address)
                        .copied()
                        .unwrap_or_else(|| {
                            vault
                                .required_tokens_u64()
                                .expect("Required tokens too large")
                        });

                let ix = spl_token::instruction::mint_to(
                    &self.state.address_finder.token_program_id,
                    &self.state.compiled_campaign.mint,
                    &vault.address,
                    &self.state.admin_keypair.pubkey(),
                    &[&self.state.admin_keypair.pubkey()],
                    funding_amount,
                )
                .expect("Failed to build mint_to ix");

                txs.push(Transaction::new(
                    &[&self.state.admin_keypair],
                    Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
                    self.latest_blockhash(),
                ));
            }
        }

        self.send_transactions(txs)
    }

    pub fn try_activate_vaults(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        for cohort in &self.state.compiled_campaign.cohorts {
            for (vault_index, vault) in cohort.vaults.iter().enumerate() {
                let expected_balance = vault
                    .required_tokens_u64()
                    .expect("Required tokens too large");

                let (ix, _, _) = build_activate_vault_v0_ix(
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
                    Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
                    self.latest_blockhash(),
                ));
            }
        }

        self.send_transactions(txs)
    }

    /// Activate vaults with custom expected balance validation, falling back to compiled campaign amounts
    ///
    /// This method allows bypassing the strict balance validation in activate_vault_v0 by
    /// specifying custom expected_balance parameters. This is crucial for edge case testing where:
    /// - Vaults were intentionally funded with non-standard amounts
    /// - Testing the interaction between funding and activation validation
    /// - Creating scenarios where vaults pass activation but fail during claims
    ///
    /// The activate_vault_v0 instruction enforces: `vault.amount == expected_balance`
    /// This method lets us satisfy that constraint with custom values.
    ///
    /// # Arguments
    /// * `custom_expected_balance` - HashMap mapping vault addresses to custom expected balance values
    ///
    /// # Example
    /// ```rust
    /// use std::collections::HashMap;
    ///
    /// // First fund with custom amounts
    /// let custom_funding = HashMap::from([(vault_addr, 1000u64)]);
    /// test.try_fund_vaults_with_custom_amounts(custom_funding)?;
    ///
    /// // Then activate expecting the same amount (bypasses validation)
    /// let custom_expectations = HashMap::from([(vault_addr, 1000u64)]);
    /// test.try_activate_vaults_with_custom_expected_balance(custom_expectations)?;
    ///
    /// // Now vault is activated but has insufficient balance for larger claims!
    /// ```
    pub fn try_activate_vaults_with_custom_expected_balance(
        &mut self,
        custom_expected_balance: HashMap<Pubkey, u64>,
    ) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        for cohort in &self.state.compiled_campaign.cohorts {
            for (vault_index, vault) in cohort.vaults.iter().enumerate() {
                // Use custom expected balance if specified, otherwise fall back to compiled campaign amount
                let expected_balance = custom_expected_balance
                    .get(&vault.address)
                    .copied()
                    .unwrap_or_else(|| {
                        vault
                            .required_tokens_u64()
                            .expect("Required tokens too large")
                    });

                let (ix, _, _) = build_activate_vault_v0_ix(
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
                    Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
                    self.latest_blockhash(),
                ));
            }
        }

        self.send_transactions(txs)
    }

    pub fn try_activate_cohorts(&mut self) -> Result<(), FailedTransactionMetadata> {
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

    pub fn try_activate_campaign(&mut self) -> Result<(), FailedTransactionMetadata> {
        // For test fixtures, we can use placeholder values
        let final_db_ipfs_hash = [1u8; 32]; // Placeholder IPFS hash
        let go_live_slot = self.current_slot() + 10; // Go live in 10 slots
        self.try_activate_campaign_with_args(Some(final_db_ipfs_hash), Some(go_live_slot))
    }

    pub fn try_activate_campaign_with_args(
        &mut self,
        final_db_ipfs_hash: Option<[u8; 32]>,
        go_live_slot: Option<u64>,
    ) -> Result<(), FailedTransactionMetadata> {
        // For test fixtures, we can use placeholder values
        let final_db_ipfs_hash = final_db_ipfs_hash.unwrap_or([1u8; 32]); // Placeholder IPFS hash
        let go_live_slot = go_live_slot.unwrap_or(self.current_slot() + 10); // Go live in 10 slots

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

    pub fn try_make_campaign_unstoppable(&mut self) -> Result<(), FailedTransactionMetadata> {
        let (ix, _, _) = build_make_campaign_unstoppable_v0_ix(
            &self.state.address_finder,
            self.state.compiled_campaign.admin,
            self.state.compiled_campaign.fingerprint,
        )
        .expect("Failed to build make campaign unstoppable v0 ix");

        let tx = Transaction::new(
            &[&self.state.admin_keypair],
            Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        Ok(())
    }

    pub fn try_pause_campaign(&mut self) -> Result<(), FailedTransactionMetadata> {
        let (ix, _, _) = build_pause_campaign_v0_ix(
            &self.state.address_finder,
            self.state.compiled_campaign.admin,
            self.state.compiled_campaign.fingerprint,
        )
        .expect("Failed to build pause campaign v0 ix");

        let tx = Transaction::new(
            &[&self.state.admin_keypair],
            Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        Ok(())
    }

    pub fn try_resume_campaign(&mut self) -> Result<(), FailedTransactionMetadata> {
        let (ix, _, _) = build_resume_campaign_v0_ix(
            &self.state.address_finder,
            self.state.compiled_campaign.admin,
            self.state.compiled_campaign.fingerprint,
        )
        .expect("Failed to build resume campaign v0 ix");

        let tx = Transaction::new(
            &[&self.state.admin_keypair],
            Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        Ok(())
    }

    pub fn try_permanently_halt_campaign(&mut self) -> Result<(), FailedTransactionMetadata> {
        let (ix, _, _) = build_permanently_halt_campaign_v0_ix(
            &self.state.address_finder,
            self.state.compiled_campaign.admin,
            self.state.compiled_campaign.fingerprint,
        )
        .expect("Failed to build permanently halt campaign v0 ix");

        let tx = Transaction::new(
            &[&self.state.admin_keypair],
            Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        Ok(())
    }

    pub fn try_claim_tokens(
        &mut self,
        claimant: &Keypair,
    ) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        let claimant_token_account = get_associated_token_address(
            &claimant.pubkey(), //
            &self.state.mint_keypair.pubkey(),
        );

        let cohorts: Vec<(CompiledCohort, ClaimLeaf)> = self
            .state
            .compiled_campaign
            .find_claimant_in_all_cohorts(&claimant.pubkey());

        // let mut vault_balances_before = HashMap::new();

        for (cohort, leaf) in cohorts {
            let merkle_proof = cohort
                .proof_for_claimant(&claimant.pubkey())
                .expect("Should be able to generate proof");

            let (ix, _, _) = build_claim_tokens_v0_ix(
                &self.state.address_finder,
                self.state.compiled_campaign.admin,
                claimant.pubkey(),
                self.state.compiled_campaign.mint,
                claimant_token_account,
                self.state.compiled_campaign.fingerprint,
                cohort.merkle_root,
                merkle_proof,
                leaf.assigned_vault_index,
                leaf.entitlements,
            )
            .expect("Failed to build claim tokens v0 ix");

            // vault_balances_before.insert(
            //     cohort.vaults[leaf.assigned_vault_index as usize].address,
            //     self.get_token_account_balance(&cohort.vaults[leaf.assigned_vault_index as usize].address)
            //         .expect("Should be able to read vault balance"),
            // );

            txs.push(Transaction::new(
                &[&claimant],
                Message::new(&[ix], Some(&claimant.pubkey())),
                self.latest_blockhash(),
            ));
        }

        self.send_transactions(txs)?;

        Ok(())
    }

    pub fn try_reclaim_tokens(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mut txs = Vec::new();

        let admin_token_account = get_associated_token_address(
            &self.state.admin_keypair.pubkey(), //
            &self.state.mint_keypair.pubkey(),
        );

        for cohort in &self.state.compiled_campaign.cohorts {
            for (vault_index, _) in cohort.vaults.iter().enumerate() {
                let (ix, _, _) = build_reclaim_tokens_v0_ix(
                    &self.state.address_finder,
                    self.state.compiled_campaign.admin,
                    admin_token_account,
                    self.state.compiled_campaign.fingerprint,
                    cohort.merkle_root,
                    vault_index.try_into().expect("Vault index too large"),
                )
                .expect("Failed to build reclaim tokens v0 ix");

                txs.push(Transaction::new(
                    &[&self.state.admin_keypair],
                    Message::new(&[ix], Some(&self.state.compiled_campaign.admin)),
                    self.latest_blockhash(),
                ));
            }
        }

        self.send_transactions(txs)?;

        Ok(())
    }

    pub fn fetch_campaign_account(&self) -> Option<CampaignV0> {
        self.svm
            .get_account(&self.state.compiled_campaign.address)
            .and_then(|a| CampaignV0::try_deserialize(&mut &a.data[..]).ok())
    }

    /// Check if an account exists
    pub fn account_exists(&self, address: &Pubkey) -> bool {
        self.svm.get_account(address).is_some()
    }

    /// Create an additional mint (useful for testing edge cases like cross-mint scenarios)
    pub fn create_ancillary_mint(
        &mut self,
        mint_keypair: &Keypair,
        decimals: u8,
    ) -> Result<(), FailedTransactionMetadata> {
        create_mint(
            &mut self.svm,
            &self.state.admin_keypair,
            mint_keypair,
            decimals,
            None,
        )?;
        Ok(())
    }

    // TODO replace this with get_token_account -> TokenAccount
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
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new(FixtureState::default(), LiteSVM::new())
            .expect("Failed to create default test fixture")
    }
}

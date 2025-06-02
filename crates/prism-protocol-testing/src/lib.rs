mod fixture_stage;
mod fixture_state;

use {
    litesvm::{
        types::{FailedTransactionMetadata, TransactionResult},
        LiteSVM,
    },
    litesvm_token::{
        get_spl_account,
        spl_token::{solana_program::native_token::LAMPORTS_PER_SOL, state::Mint},
        CreateMint, MintTo,
    },
    prism_protocol_sdk::{
        build_activate_campaign_v0_ix, build_activate_cohort_v0_ix, build_activate_vault_v0_ix,
        build_initialize_campaign_v0_ix, build_initialize_cohort_v0_ix,
        build_initialize_vault_v0_ix, AddressFinder,
    },
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_sysvar::clock::Clock,
    solana_transaction::Transaction,
    std::env,
};

pub use fixture_stage::FixtureStage;
pub use fixture_state::FixtureState;

/// Load the Prism Protocol program into LiteSVM
/// Note: build.rs ensures that the program is built before this is called
pub fn load_prism_protocol(svm: &mut LiteSVM, program_id: Pubkey) {
    svm.add_program(
        program_id,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/prism_protocol.so"
        )),
    );
}

pub struct TestFixture {
    pub address_finder: AddressFinder,
    pub admin: Pubkey,
    admin_keypair: Keypair,
    state: FixtureState,
    svm: LiteSVM,
    logging_enabled: bool,
    // Counters for generating incremental values
    fingerprint_counter: u8,
    cohort_counter: u8,
    vault_counter: u8,
}

impl TestFixture {
    pub fn new(
        address_finder: AddressFinder,
        admin_keypair: Keypair,
        state: FixtureState,
        mut svm: LiteSVM,
    ) -> Self {
        let admin_address = admin_keypair.pubkey();

        // Load the Prism Protocol program at runtime
        load_prism_protocol(&mut svm, address_finder.program_id);

        svm.airdrop(&admin_address, LAMPORTS_PER_SOL * 10)
            .expect("Failed to airdrop SOL to admin");

        Self {
            address_finder,
            admin: admin_address,
            admin_keypair,
            state,
            svm,
            logging_enabled: true,
            // Counters for generating incremental values
            fingerprint_counter: 0,
            cohort_counter: 0,
            vault_counter: 0,
        }
    }

    pub fn airdrop(&mut self, to: &Pubkey, amount: u64) -> TransactionResult {
        self.svm.airdrop(to, amount)
    }

    pub fn create_mint(
        &mut self,
        decimals: u8,
    ) -> Result<(Pubkey, Mint), FailedTransactionMetadata> {
        let mint_address = CreateMint::new(&mut self.svm, &self.admin_keypair)
            .authority(&self.admin)
            .decimals(decimals)
            .send()?;

        let mint: Mint = get_spl_account(&mut self.svm, &mint_address)?;

        Ok((mint_address, mint))
    }

    pub fn latest_slot(&self) -> u64 {
        self.svm.get_sysvar::<Clock>().slot
    }

    pub fn latest_blockhash(&self) -> solana_hash::Hash {
        self.svm.latest_blockhash()
    }

    pub fn warp_to_slot(&mut self, slot: u64) {
        self.svm.warp_to_slot(slot);
    }

    /// Send a transaction and optionally print logs based on the logging_enabled setting
    pub fn send_transaction(&mut self, tx: Transaction) -> TransactionResult {
        let result = self.svm.send_transaction(tx);

        if self.logging_enabled {
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

    pub fn mint_to(
        &mut self,
        mint: Pubkey,
        recipient: Pubkey,
        amount: u64,
    ) -> Result<(), FailedTransactionMetadata> {
        MintTo::new(
            &mut self.svm,
            &self.admin_keypair,
            &mint,
            &recipient,
            amount,
        )
        .send()
    }

    /// Enable transaction logging (default: enabled)
    pub fn enable_logging(&mut self) {
        self.logging_enabled = true;
    }

    /// Disable transaction logging
    pub fn disable_logging(&mut self) {
        self.logging_enabled = false;
    }

    pub fn jump_to(
        &mut self,
        target_stage: FixtureStage,
    ) -> Result<FixtureState, FailedTransactionMetadata> {
        // step all stages:
        // - less than or equal to target stage
        // - greater than current stage (if any)
        let all_stages = [
            FixtureStage::CampaignInitialized,
            FixtureStage::CohortInitialized,
            FixtureStage::VaultInitialized,
            FixtureStage::VaultActivated,
            FixtureStage::CohortActivated,
            FixtureStage::CampaignActivated,
        ];

        let stages_to_step: Vec<FixtureStage> = all_stages
            .into_iter()
            .filter(|s| s <= &target_stage)
            .filter(|s| match &self.state.stage {
                Some(current) => s > current,
                None => true,
            })
            .collect();

        for stage in stages_to_step {
            self.step_to(stage)?;
        }

        Ok(self.state.clone())
    }

    pub fn step_to(&mut self, stage: FixtureStage) -> Result<(), FailedTransactionMetadata> {
        match stage {
            FixtureStage::CampaignInitialized => {
                self.advance_to_campaign_initialized()?;
            }
            FixtureStage::CohortInitialized => {
                self.advance_to_cohort_initialized()?;
            }
            FixtureStage::VaultInitialized => {
                self.advance_to_vault_initialized()?;
            }
            FixtureStage::VaultActivated => {
                self.advance_to_vault_activated()?;
            }
            FixtureStage::CohortActivated => {
                self.advance_to_cohort_activated()?;
            }
            FixtureStage::CampaignActivated => {
                self.advance_to_campaign_activated()?;
            }
        }
        Ok(())
    }

    fn advance_to_campaign_initialized(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mint = self.next_mint()?;
        let campaign_fingerprint = self.next_campaign_fingerprint();

        let (ix, accounts, _) = build_initialize_campaign_v0_ix(
            &self.address_finder,
            self.admin,
            campaign_fingerprint,
            mint,
            1, // expected_cohort_count,
        )
        .expect("Failed to build initialize campaign v0 ix");

        let tx = Transaction::new(
            &[&self.admin_keypair],
            Message::new(&[ix], Some(&self.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        self.state = FixtureState {
            mint: Some(mint),
            campaign_fingerprint: Some(campaign_fingerprint),
            campaign: Some(accounts.campaign),
            stage: Some(FixtureStage::CampaignInitialized),
            ..self.state
        };

        Ok(())
    }

    fn advance_to_cohort_initialized(&mut self) -> Result<(), FailedTransactionMetadata> {
        let campaign_fingerprint = self.state.campaign_fingerprint.expect(
            "Campaign fingerprint not initialized.
            Did you forget to call `advance_to_campaign_initialized`?",
        );

        let cohort_merkle_root = self.next_cohort_merkle_root();

        let (ix, accounts, _) = build_initialize_cohort_v0_ix(
            &self.address_finder,
            self.admin,
            campaign_fingerprint,
            cohort_merkle_root,
            self.next_amount_per_entitlement(),
            1, // expected_vault_count,
        )
        .expect("Failed to build initialize cohort v0 ix");

        let tx = Transaction::new(
            &[&self.admin_keypair],
            Message::new(&[ix], Some(&self.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        self.state = FixtureState {
            cohort_merkle_root: Some(cohort_merkle_root),
            cohort: Some(accounts.cohort),
            stage: Some(FixtureStage::CohortInitialized),
            ..self.state
        };

        Ok(())
    }

    fn advance_to_vault_initialized(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mint = self.state.mint.expect(
            "Mint not initialized.
            Did you forget to call `advance_to_campaign_initialized`?",
        );

        let campaign_fingerprint = self.state.campaign_fingerprint.expect(
            "Campaign fingerprint not initialized.
            Did you forget to call `advance_to_campaign_initialized`?",
        );

        let cohort_merkle_root = self.state.cohort_merkle_root.expect(
            "Cohort merkle root not initialized.
            Did you forget to call `advance_to_cohort_initialized`?",
        );

        let (ix, accounts, _) = build_initialize_vault_v0_ix(
            &self.address_finder,
            self.admin,
            campaign_fingerprint,
            cohort_merkle_root,
            mint,
            0, // vault_index (0-based indexing),
        )
        .expect("Failed to build initialize vault v0 ix");

        let tx = Transaction::new(
            &[&self.admin_keypair],
            Message::new(&[ix], Some(&self.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        self.state = FixtureState {
            vault: Some(accounts.vault),
            stage: Some(FixtureStage::VaultInitialized),
            ..self.state
        };

        Ok(())
    }

    fn advance_to_vault_activated(&mut self) -> Result<(), FailedTransactionMetadata> {
        let mint = self.state.mint.expect(
            "Mint not initialized.
            Did you forget to call `advance_to_campaign_initialized`?",
        );

        let campaign_fingerprint = self.state.campaign_fingerprint.expect(
            "Campaign fingerprint not initialized.
            Did you forget to call `advance_to_campaign_initialized`?",
        );

        let cohort_merkle_root = self.state.cohort_merkle_root.expect(
            "Cohort merkle root not initialized.
            Did you forget to call `advance_to_cohort_initialized`?",
        );

        let vault = self.state.vault.expect(
            "Vault not initialized.
            Did you forget to call `advance_to_vault_initialized`?",
        );

        let expected_balance = self.next_expected_balance();

        // Fund the vault with the expected balance before activating
        self.mint_to(mint, vault, expected_balance)?;

        let (ix, _, _) = build_activate_vault_v0_ix(
            &self.address_finder,
            self.admin,
            campaign_fingerprint,
            cohort_merkle_root,
            0, // vault_index (0-based indexing),
            expected_balance,
        )
        .expect("Failed to build activate vault v0 ix");

        let tx = Transaction::new(
            &[&self.admin_keypair],
            Message::new(&[ix], Some(&self.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        self.state = FixtureState {
            vault: Some(vault),
            vault_expected_balance: Some(expected_balance),
            stage: Some(FixtureStage::VaultActivated),
            ..self.state
        };

        Ok(())
    }

    fn advance_to_cohort_activated(&mut self) -> Result<(), FailedTransactionMetadata> {
        let campaign_fingerprint = self.state.campaign_fingerprint.expect(
            "Campaign fingerprint not initialized.
            Did you forget to call `advance_to_campaign_initialized`?",
        );

        let cohort_merkle_root = self.state.cohort_merkle_root.expect(
            "Cohort merkle root not initialized.
            Did you forget to call `advance_to_cohort_initialized`?",
        );

        let (ix, _, _) = build_activate_cohort_v0_ix(
            &self.address_finder,
            self.admin,
            campaign_fingerprint,
            cohort_merkle_root,
        )
        .expect("Failed to build activate cohort v0 ix");

        let tx = Transaction::new(
            &[&self.admin_keypair],
            Message::new(&[ix], Some(&self.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        self.state = FixtureState {
            stage: Some(FixtureStage::CohortActivated),
            ..self.state
        };

        Ok(())
    }

    fn advance_to_campaign_activated(&mut self) -> Result<(), FailedTransactionMetadata> {
        let campaign_fingerprint = self.state.campaign_fingerprint.expect(
            "Campaign fingerprint not initialized.
            Did you forget to call `advance_to_campaign_initialized`?",
        );

        let go_live_slot = self.next_go_live_slot();
        let final_db_ipfs_hash = [1u8; 32]; // Use non-empty hash (program rejects all zeros)

        let (ix, _, _) = build_activate_campaign_v0_ix(
            &self.address_finder,
            self.admin,
            campaign_fingerprint,
            final_db_ipfs_hash,
            go_live_slot,
        )
        .expect("Failed to build activate campaign v0 ix");

        let tx = Transaction::new(
            &[&self.admin_keypair],
            Message::new(&[ix], Some(&self.admin)),
            self.latest_blockhash(),
        );

        self.send_transaction(tx)?;

        self.state = FixtureState {
            go_live_slot: Some(go_live_slot),
            stage: Some(FixtureStage::CampaignActivated),
            ..self.state
        };

        Ok(())
    }

    /// Generate next campaign fingerprint (incremental)
    fn next_campaign_fingerprint(&mut self) -> [u8; 32] {
        self.fingerprint_counter += 1;
        [self.fingerprint_counter; 32]
    }

    /// Generate next cohort merkle root (incremental)
    fn next_cohort_merkle_root(&mut self) -> [u8; 32] {
        self.cohort_counter += 1;
        [self.cohort_counter; 32]
    }

    /// Generate next mint (requires SVM transaction)
    fn next_mint(&mut self) -> Result<Pubkey, FailedTransactionMetadata> {
        let (mint, _) = self.create_mint(9)?; // 9 decimals
        Ok(mint)
    }

    /// Generate next amount per entitlement (fixed for now)
    fn next_amount_per_entitlement(&self) -> u64 {
        1_000_000_000 // 1 token with 9 decimals
    }

    /// Generate next expected balance (fixed for now)
    fn next_expected_balance(&self) -> u64 {
        10_000_000_000 // 10 tokens with 9 decimals
    }

    /// Generate next go live slot (future slot)
    fn next_go_live_slot(&self) -> u64 {
        self.latest_slot() + 10
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new(
            AddressFinder::default(),
            Keypair::new(),
            FixtureState::default(),
            LiteSVM::new(),
        )
    }
}

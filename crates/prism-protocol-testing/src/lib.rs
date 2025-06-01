use {
    anchor_lang::Space,
    anchor_spl::token::{spl_token, Mint, ID as TOKEN_PROGRAM_ID},
    mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk},
    prism_protocol::{state::CampaignV0, ID as PRISM_PROGRAM_ID},
    prism_protocol_merkle::{create_merkle_tree, ClaimTree},
    prism_protocol_sdk::{
        build_initialize_campaign_v0_ix, build_initialize_cohort_v0_ix, build_initialize_vault_v0_ix,
        AddressFinder,
    },
    solana_sdk::{
        account::Account as SolanaAccount,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program::ID as SYSTEM_PROGRAM_ID,
    },
};

/// Standard test constants
pub const TEST_ADMIN_LAMPORTS: u64 = 1_000_000_000; // 1 SOL
pub const TEST_AMOUNT_PER_ENTITLEMENT: u64 = 1_000_000_000; // 1 token (assuming 9 decimals)

/// Well-known test claimants for consistent testing
pub struct TestClaimants {
    pub alice: Pubkey,
    pub bob: Pubkey, 
    pub charlie: Pubkey,
    pub diana: Pubkey,
    pub eve: Pubkey,
}

impl TestClaimants {
    pub fn new() -> Self {
        Self {
            alice: Pubkey::from([1u8; 32]),
            bob: Pubkey::from([2u8; 32]),
            charlie: Pubkey::from([3u8; 32]),
            diana: Pubkey::from([4u8; 32]),
            eve: Pubkey::from([5u8; 32]),
        }
    }

    pub fn all(&self) -> Vec<Pubkey> {
        vec![self.alice, self.bob, self.charlie, self.diana, self.eve]
    }

    pub fn small_group(&self) -> Vec<Pubkey> {
        vec![self.alice, self.bob]
    }

    pub fn medium_group(&self) -> Vec<Pubkey> {
        vec![self.alice, self.bob, self.charlie]
    }
}

/// Generate a meaningful test fingerprint based on test name
pub fn generate_test_fingerprint(test_name: &str) -> [u8; 32] {
    let mut fingerprint = [0u8; 32];
    let test_bytes = test_name.as_bytes();
    
    // Simple deterministic fingerprint generation
    // Copy test name bytes, cycling if needed
    for (i, &byte) in test_bytes.iter().cycle().take(32).enumerate() {
        fingerprint[i] = byte.wrapping_add(i as u8); // Add position for uniqueness
    }
    
    fingerprint
}

/// Test fixture containing common test setup
pub struct TestFixture {
    pub mollusk: Mollusk,
    pub admin_keypair: Keypair,
    pub admin_address: Pubkey,
    pub test_fingerprint: [u8; 32],
    pub address_finder: AddressFinder,
}

impl TestFixture {
    /// Create a new test fixture with standard setup
    pub fn new() -> Self {
        let mut mollusk = Mollusk::new(&PRISM_PROGRAM_ID, "prism_protocol");
        mollusk_svm_programs_token::token::add_program(&mut mollusk);
        mollusk_svm_programs_token::associated_token::add_program(&mut mollusk);

        let admin_keypair = Keypair::new();
        let admin_address = admin_keypair.pubkey();
        let test_fingerprint = generate_test_fingerprint("test_fixture");

        Self {
            mollusk,
            admin_keypair,
            admin_address,
            test_fingerprint,
            address_finder: AddressFinder::default(),
        }
    }

    /// Create a new SPL token mint for testing
    pub fn create_mint(&mut self, mint_keypair: &Keypair, decimals: u8) -> SolanaAccount {
        let mint_authority = self.admin_address;
        let initialize_mint_ix = spl_token::instruction::initialize_mint2(
            &TOKEN_PROGRAM_ID,
            &mint_keypair.pubkey(),
            &mint_authority,
            None, // No freeze authority
            decimals,
        )
        .expect("Failed to create initialize_mint2 instruction");

        let mint_account = SolanaAccount {
            lamports: 1_461_600, // Rent-exempt amount for mint
            data: vec![0u8; Mint::LEN],
            owner: TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        };

        let result = self.mollusk.process_and_validate_instruction(
            &initialize_mint_ix,
            &[(mint_keypair.pubkey(), mint_account.clone())],
            &[Check::success()],
        );

        println!("✅ Mint {} initialized successfully", mint_keypair.pubkey());

        result
            .get_account(&mint_keypair.pubkey())
            .expect("Mint account not found after initialization")
            .clone()
    }

    /// Initialize a campaign and return the campaign account data
    pub fn initialize_campaign_v0(&mut self, mint: Pubkey, expected_cohort_count: u8) -> InitializedCampaign {
        let (campaign_address, campaign_bump) = self
            .address_finder
            .find_campaign_v0_address(&self.admin_address, &self.test_fingerprint);

        let (initialize_campaign_ix, _, _) = build_initialize_campaign_v0_ix(
            &self.address_finder,
            self.admin_address,
            self.test_fingerprint,
            mint,
            expected_cohort_count,
        )
        .expect("Failed to build initialize_campaign instruction");

        let keyed_account_for_admin = (
            self.admin_address,
            SolanaAccount::new(TEST_ADMIN_LAMPORTS, 0, &SYSTEM_PROGRAM_ID),
        );

        let keyed_account_for_campaign = (
            campaign_address,
            SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID),
        );

        println!(
            "Initializing campaign: {} (bump: {}, size: {}, admin: {})",
            campaign_address,
            campaign_bump,
            CampaignV0::INIT_SPACE + 8,
            self.admin_address,
        );

        let result = self.mollusk.process_and_validate_instruction(
            &initialize_campaign_ix,
            &[
                keyed_account_for_system_program(),
                keyed_account_for_admin,
                keyed_account_for_campaign,
            ],
            &[Check::success()],
        );

        println!(
            "Campaign initialized - CU consumed: {}, execution time: {}",
            result.compute_units_consumed, result.execution_time
        );

        let admin_account = result
            .get_account(&self.admin_address)
            .expect("Admin account not found")
            .clone();

        let campaign_account = result
            .get_account(&campaign_address)
            .expect("Campaign account not found")
            .clone();

        InitializedCampaign {
            address: campaign_address,
            bump: campaign_bump,
            admin_account,
            campaign_account,
            mint,
        }
    }

    /// Initialize a cohort with a real merkle tree and return the cohort data
    pub fn initialize_cohort_v0(
        &mut self,
        campaign: &InitializedCampaign,
        claimants: &[Pubkey],
        expected_vault_count: u8,
        amount_per_entitlement: u64,
    ) -> InitializedCohort {
        // Create claimant entitlements pairs
        let claimant_entitlements: Vec<(Pubkey, u64)> = claimants
            .iter()
            .map(|&claimant| (claimant, amount_per_entitlement))
            .collect();

        // Create a real merkle tree using production function
        let merkle_tree = create_merkle_tree(&claimant_entitlements, expected_vault_count as usize)
            .expect("Failed to create merkle tree");

        let merkle_root = merkle_tree.root().expect("Failed to get merkle root");

        // Derive cohort address
        let (cohort_address, cohort_bump) = self
            .address_finder
            .find_cohort_v0_address(&campaign.address, &merkle_root);

        // Build cohort initialization instruction
        let (initialize_cohort_ix, _, _) = build_initialize_cohort_v0_ix(
            &self.address_finder,
            self.admin_address,
            self.test_fingerprint,
            merkle_root,
            amount_per_entitlement,
            expected_vault_count,
        )
        .expect("Failed to build initialize_cohort instruction");

        println!(
            "Initializing cohort: {} (bump: {}, merkle_root: {:?}, vaults: {})",
            cohort_address, cohort_bump, merkle_root, expected_vault_count
        );

        // Execute cohort initialization
        let result = self.mollusk.process_and_validate_instruction(
            &initialize_cohort_ix,
            &[
                keyed_account_for_system_program(),
                (self.admin_address, campaign.admin_account.clone()),
                (campaign.address, campaign.campaign_account.clone()),
                (cohort_address, SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID)),
            ],
            &[Check::success()],
        );

        println!(
            "Cohort initialized - CU consumed: {}, execution time: {}",
            result.compute_units_consumed, result.execution_time
        );

        let cohort_account = result
            .get_account(&cohort_address)
            .expect("Cohort account not found")
            .clone();

        InitializedCohort {
            address: cohort_address,
            bump: cohort_bump,
            merkle_tree,
            cohort_account,
        }
    }

    /// Create a vault using the proper create_vault instruction
    pub fn initialize_vault_v0(
        &mut self,
        campaign: &InitializedCampaign,
        cohort: &mut InitializedCohort,
        vault_index: u8,
        mint_account: &SolanaAccount,
    ) -> (Pubkey, SolanaAccount) {
        let (vault_address, _vault_bump) = self
            .address_finder
            .find_vault_v0_address(&cohort.address, vault_index);

        println!(
            "Creating vault {} at address {}",
            vault_index, vault_address
        );

        // Build create vault instruction
        let (initialize_vault_ix, _, _) = build_initialize_vault_v0_ix(
            &self.address_finder,
            self.admin_address,
            self.test_fingerprint,
            cohort
                .merkle_tree
                .root()
                .expect("Failed to get merkle root"),
            campaign.mint,
            vault_index,
        )
        .expect("Failed to build create vault instruction");

        // Execute create vault instruction
        let result = self.mollusk.process_and_validate_instruction(
            &initialize_vault_ix,
            &[
                keyed_account_for_system_program(),
                (self.admin_address, campaign.admin_account.clone()),
                (campaign.address, campaign.campaign_account.clone()),
                (cohort.address, cohort.cohort_account.clone()),
                (campaign.mint, mint_account.clone()),
                (vault_address, SolanaAccount::new(0, 0, &SYSTEM_PROGRAM_ID)),
                mollusk_svm_programs_token::token::keyed_account(),
            ],
            &[Check::success()],
        );

        println!(
            "Vault created successfully - CU consumed: {}, execution time: {}",
            result.compute_units_consumed, result.execution_time
        );

        // Update the cohort account with the new vault
        cohort.cohort_account = result
            .get_account(&cohort.address)
            .expect("Cohort account not found after vault creation")
            .clone();

        // Get the created vault account
        let vault_account = result
            .get_account(&vault_address)
            .expect("Vault account not found")
            .clone();

        (vault_address, vault_account)
    }

    /// Fund a vault with tokens (mint tokens to the vault)
    pub fn fund_vault(
        &mut self,
        mint: Pubkey,
        mint_account: &SolanaAccount,
        vault_address: Pubkey,
        vault_account: &SolanaAccount,
        amount: u64,
    ) -> SolanaAccount {
        let mint_to_vault_ix = spl_token::instruction::mint_to(
            &TOKEN_PROGRAM_ID,
            &mint,
            &vault_address,
            &self.admin_address, // Admin is mint authority
            &[],
            amount,
        )
        .expect("Failed to create mint_to instruction");

        let result = self.mollusk.process_and_validate_instruction(
            &mint_to_vault_ix,
            &[
                (mint, mint_account.clone()),
                (vault_address, vault_account.clone()),
                (
                    self.admin_address,
                    SolanaAccount::new(1_000_000_000, 0, &SYSTEM_PROGRAM_ID),
                ),
            ],
            &[Check::success()],
        );

        println!("✅ Vault {} funded with {} tokens", vault_address, amount);

        result
            .get_account(&vault_address)
            .expect("Vault account not found after funding")
            .clone()
    }

    /// Set up campaign to a specific lifecycle stage
    pub fn setup_to_stage(&mut self, target_stage: CampaignLifecycleStage) -> CampaignTestState {
        match target_stage {
            CampaignLifecycleStage::CampaignInitialized => self.setup_campaign_initialized(),
            CampaignLifecycleStage::CohortsInitialized => self.setup_cohorts_initialized(),
            CampaignLifecycleStage::VaultsInitialized => self.setup_vaults_initialized(),
            CampaignLifecycleStage::VaultsActivated => self.setup_vaults_activated(),
            CampaignLifecycleStage::CohortsActivated => self.setup_cohorts_activated(),
            CampaignLifecycleStage::CampaignActivated => self.setup_campaign_activated(),
        }
    }

    /// Execute an action and expect it to succeed
    pub fn expect_success(&mut self, state: &mut CampaignTestState, action: CampaignAction) {
        match self.try_action(state, action) {
            Ok(()) => {}, // Success as expected
            Err(error) => panic!("Expected action to succeed but got error: {:?}", error),
        }
    }

    /// Execute an action and expect it to fail with a specific error
    pub fn expect_failure(&mut self, state: &mut CampaignTestState, action: CampaignAction, expected_error: &str) {
        match self.try_action(state, action) {
            Ok(()) => panic!("Expected action to fail with '{}' but it succeeded", expected_error),
            Err(error) => {
                assert!(
                    error.contains(expected_error),
                    "Expected error containing '{}' but got: {}",
                    expected_error,
                    error
                );
            }
        }
    }

    /// Advance state to the next lifecycle stage
    pub fn advance_to_stage(&mut self, state: &mut CampaignTestState, target_stage: CampaignLifecycleStage) {
        // This is a simplified implementation - in practice we'd validate transitions
        let mut new_state = self.setup_to_stage(target_stage);
        
        // Preserve existing data where possible
        new_state.campaign = state.campaign.clone();
        new_state.cohorts = state.cohorts.clone();
        new_state.vaults = state.vaults.clone();
        new_state.mint_account = state.mint_account.clone();
        new_state.funded_vaults = state.funded_vaults.clone();
        
        *state = new_state;
    }

    // Private helper methods for setting up each stage
    fn setup_campaign_initialized(&mut self) -> CampaignTestState {
        // Create test mint
        let mint_keypair = Keypair::new();
        let mint_account = self.create_mint(&mint_keypair, 9);
        let mint = mint_keypair.pubkey();

        // Initialize campaign with expected cohort count of 2
        let campaign = self.initialize_campaign_v0(mint, 2);

        CampaignTestState {
            stage: CampaignLifecycleStage::CampaignInitialized,
            campaign,
            cohorts: vec![],
            vaults: vec![],
            mint_account,
            funded_vaults: vec![],
        }
    }

    fn setup_cohorts_initialized(&mut self) -> CampaignTestState {
        let mut state = self.setup_campaign_initialized();

        // Add two cohorts with different configurations
        let claimants_1 = vec![Pubkey::new_unique(), Pubkey::new_unique()];
        let cohort_1 = self.initialize_cohort_v0(
            &state.campaign,
            &claimants_1,
            2, // 2 vaults
            1_000_000_000, // 1 token per entitlement
        );

        let claimants_2 = vec![Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::new_unique()];
        let cohort_2 = self.initialize_cohort_v0(
            &state.campaign,
            &claimants_2,
            3, // 3 vaults
            500_000_000, // 0.5 tokens per entitlement
        );

        state.cohorts = vec![cohort_1, cohort_2];
        state.vaults = vec![vec![], vec![]];
        state.funded_vaults = vec![vec![], vec![]];
        state.stage = CampaignLifecycleStage::CohortsInitialized;

        state
    }

    fn setup_vaults_initialized(&mut self) -> CampaignTestState {
        let mut state = self.setup_cohorts_initialized();

        // Create vaults for each cohort
        for cohort_index in 0..state.cohorts.len() {
            let cohort = &mut state.cohorts[cohort_index];
            let expected_vault_count = if cohort_index == 0 { 2 } else { 3 };
            
            for vault_index in 0..expected_vault_count {
                let (vault_address, vault_account) = self.initialize_vault_v0(
                    &state.campaign,
                    cohort,
                    vault_index,
                    &state.mint_account,
                );
                
                if state.vaults.len() <= cohort_index {
                    state.vaults.resize(cohort_index + 1, vec![]);
                }
                state.vaults[cohort_index].push((vault_address, vault_account));
                
                if state.funded_vaults.len() <= cohort_index {
                    state.funded_vaults.resize(cohort_index + 1, vec![]);
                }
                state.funded_vaults[cohort_index].push(false);
            }
        }

        state.stage = CampaignLifecycleStage::VaultsInitialized;
        state
    }

    fn setup_vaults_activated(&mut self) -> CampaignTestState {
        let mut state = self.setup_vaults_initialized();

        // Fund all vaults
        for cohort_index in 0..state.vaults.len() {
            for vault_index in 0..state.vaults[cohort_index].len() {
                let (vault_address, _) = &state.vaults[cohort_index][vault_index];
                let vault_account = &state.vaults[cohort_index][vault_index].1;
                
                // Fund with enough tokens for claims
                let amount = 10_000_000_000; // 10 tokens
                let updated_vault_account = self.fund_vault(
                    state.campaign.mint,
                    &state.mint_account,
                    *vault_address,
                    vault_account,
                    amount,
                );
                
                state.vaults[cohort_index][vault_index].1 = updated_vault_account;
                state.funded_vaults[cohort_index][vault_index] = true;
            }
        }

        state.stage = CampaignLifecycleStage::VaultsActivated;
        state
    }

    fn setup_cohorts_activated(&mut self) -> CampaignTestState {
        let mut state = self.setup_vaults_activated();
        
        // TODO: Implement cohort activation when we have the instruction
        // For now, we'll assume vaults activated means cohorts are ready
        
        state.stage = CampaignLifecycleStage::CohortsActivated;
        state
    }

    fn setup_campaign_activated(&mut self) -> CampaignTestState {
        let mut state = self.setup_cohorts_activated();
        
        // TODO: Implement campaign activation when we have the instruction
        // For now, we'll assume the campaign is ready for claims
        
        state.stage = CampaignLifecycleStage::CampaignActivated;
        state
    }

    // Helper method to attempt an action and return result
    fn try_action(&mut self, state: &mut CampaignTestState, action: CampaignAction) -> Result<(), String> {
        match action {
            CampaignAction::InitializeCohort { .. } => {
                // TODO: Implement when we have versioned method names
                Err("InitializeCohort not yet implemented".to_string())
            }
            CampaignAction::ClaimTokens { claimant: _ } => {
                // TODO: Implement claim flow
                // For now, just check if campaign is activated
                if state.stage == CampaignLifecycleStage::CampaignActivated {
                    Ok(())
                } else {
                    Err("Campaign not activated".to_string())
                }
            }
            _ => Err("Action not yet implemented".to_string()),
        }
    }
}

/// Result of campaign initialization
#[derive(Clone)]
pub struct InitializedCampaign {
    pub address: Pubkey,
    pub bump: u8,
    pub admin_account: SolanaAccount,
    pub campaign_account: SolanaAccount,
    pub mint: Pubkey,
}

/// Result of cohort initialization with merkle tree
#[derive(Clone)]
pub struct InitializedCohort {
    pub address: Pubkey,
    pub bump: u8,
    pub merkle_tree: ClaimTree,
    pub cohort_account: SolanaAccount,
}

/// Campaign lifecycle stages for state machine testing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignLifecycleStage {
    /// Campaign has been initialized but is inactive
    CampaignInitialized,
    /// Cohorts have been initialized and added to campaign
    CohortsInitialized,
    /// Vaults have been created for cohorts but are empty
    VaultsInitialized,
    /// Vaults have been funded and activated
    VaultsActivated,
    /// Cohorts have been activated
    CohortsActivated,
    /// Campaign has been activated and claims are allowed
    CampaignActivated,
}

/// Actions that can be attempted in the campaign lifecycle
#[derive(Debug, Clone)]
pub enum CampaignAction {
    /// Initialize a new cohort
    InitializeCohort { claimants: Vec<Pubkey>, vault_count: u8, amount_per_entitlement: u64 },
    /// Initialize a vault for a cohort
    InitializeVault { cohort_index: usize, vault_index: u8 },
    /// Fund a vault with tokens
    FundVault { cohort_index: usize, vault_index: u8, amount: u64 },
    /// Activate a vault
    ActivateVault { cohort_index: usize, vault_index: u8 },
    /// Activate a cohort
    ActivateCohort { cohort_index: usize },
    /// Activate the campaign
    ActivateCampaign,
    /// Claim tokens for a claimant
    ClaimTokens { claimant: Pubkey },
    /// Pause the campaign
    PauseCampaign,
    /// Resume the campaign
    ResumeCampaign,
    /// Permanently halt the campaign
    PermanentlyHaltCampaign,
}

/// Complete campaign setup state for testing
pub struct CampaignTestState {
    pub stage: CampaignLifecycleStage,
    pub campaign: InitializedCampaign,
    pub cohorts: Vec<InitializedCohort>,
    pub vaults: Vec<Vec<(Pubkey, SolanaAccount)>>, // vaults[cohort_index][vault_index]
    pub mint_account: SolanaAccount,
    pub funded_vaults: Vec<Vec<bool>>, // funded_vaults[cohort_index][vault_index]
}

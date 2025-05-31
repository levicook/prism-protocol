use {
    anchor_lang::Space,
    anchor_spl::token::{spl_token, Mint, ID as TOKEN_PROGRAM_ID},
    mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk},
    prism_protocol::{state::CampaignV0, ID as PRISM_PROGRAM_ID},
    prism_protocol_merkle::{create_merkle_tree, ClaimTree},
    prism_protocol_sdk::{
        build_create_vault_ix, build_initialize_campaign_ix, build_initialize_cohort_ix,
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

/// Generate a test merkle root (all zeros for simplicity)
pub fn generate_test_merkle_root() -> [u8; 32] {
    [0; 32]
}

/// Generate a test fingerprint (all ones for simplicity)
pub fn generate_test_fingerprint() -> [u8; 32] {
    [1; 32]
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
        let test_fingerprint = generate_test_fingerprint();

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
    pub fn initialize_campaign(&mut self, mint: Pubkey) -> InitializedCampaign {
        let (campaign_address, campaign_bump) = self
            .address_finder
            .find_campaign_v0_address(&self.admin_address, &self.test_fingerprint);

        let (initialize_campaign_ix, _, _) = build_initialize_campaign_ix(
            self.admin_address,
            campaign_address,
            self.test_fingerprint,
            mint,
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

    /// Activate a campaign (set is_active to true)
    pub fn activate_campaign(&mut self, campaign: &mut InitializedCampaign) {
        let (set_active_ix, _, _) = prism_protocol_sdk::build_set_campaign_active_status_ix(
            self.admin_address,
            campaign.address,
            self.test_fingerprint,
            true, // Set to active
        )
        .expect("Failed to build set_campaign_active_status instruction");

        let result = self.mollusk.process_and_validate_instruction(
            &set_active_ix,
            &[
                (self.admin_address, campaign.admin_account.clone()),
                (campaign.address, campaign.campaign_account.clone()),
            ],
            &[Check::success()],
        );

        println!(
            "Campaign activated - CU consumed: {}, execution time: {}",
            result.compute_units_consumed, result.execution_time
        );

        // Update the campaign account with the new state
        campaign.campaign_account = result
            .get_account(&campaign.address)
            .expect("Campaign account not found after activation")
            .clone();
    }

    /// Initialize a cohort with a real merkle tree and return the cohort data
    pub fn initialize_cohort_with_merkle_tree(
        &mut self,
        campaign: &InitializedCampaign,
        claimants: &[Pubkey],
        vault_count: usize,
        amount_per_entitlement: u64,
    ) -> InitializedCohort {
        // Create claimant entitlements pairs
        let claimant_entitlements: Vec<(Pubkey, u64)> = claimants
            .iter()
            .map(|&claimant| (claimant, amount_per_entitlement))
            .collect();

        // Create a real merkle tree using production function
        let merkle_tree = create_merkle_tree(&claimant_entitlements, vault_count)
            .expect("Failed to create merkle tree");

        let merkle_root = merkle_tree.root().expect("Failed to get merkle root");

        // Derive cohort address
        let (cohort_address, cohort_bump) = self
            .address_finder
            .find_cohort_v0_address(&campaign.address, &merkle_root);

        // Build cohort initialization instruction
        let (initialize_cohort_ix, _, _) = build_initialize_cohort_ix(
            self.admin_address,
            campaign.address,
            self.test_fingerprint,
            cohort_address,
            merkle_root,
            amount_per_entitlement,
            vault_count as u8,
        )
        .expect("Failed to build initialize_cohort instruction");

        println!(
            "Initializing cohort: {} (bump: {}, merkle_root: {:?}, vaults: {})",
            cohort_address, cohort_bump, merkle_root, vault_count
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
    pub fn create_vault(
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
        let (create_vault_ix, _, _) = build_create_vault_ix(
            self.admin_address,
            campaign.address,
            cohort.address,
            campaign.mint,
            vault_address,
            self.test_fingerprint,
            cohort
                .merkle_tree
                .root()
                .expect("Failed to get merkle root"),
            vault_index,
        )
        .expect("Failed to build create vault instruction");

        // Execute create vault instruction
        let result = self.mollusk.process_and_validate_instruction(
            &create_vault_ix,
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
}

/// Result of campaign initialization
pub struct InitializedCampaign {
    pub address: Pubkey,
    pub bump: u8,
    pub admin_account: SolanaAccount,
    pub campaign_account: SolanaAccount,
    pub mint: Pubkey,
}

/// Result of cohort initialization with merkle tree
pub struct InitializedCohort {
    pub address: Pubkey,
    pub bump: u8,
    pub merkle_tree: ClaimTree,
    pub cohort_account: SolanaAccount,
}

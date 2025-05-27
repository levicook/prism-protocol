use {
    anchor_lang::Space,
    mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk},
    prism_protocol::{state::CampaignV0, ID as PRISM_PROGRAM_ID},
    prism_protocol_merkle::{create_merkle_tree, ClaimMerkleTree},
    prism_protocol_sdk::{
        address_finders::{find_campaign_address, find_cohort_v0_address},
        instruction_builders::{build_initialize_campaign_ix, build_initialize_cohort_ix},
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

/// Generate test vaults (just unique pubkeys for testing)
pub fn generate_test_vaults(count: usize) -> Vec<Pubkey> {
    (0..count).map(|_| Pubkey::new_unique()).collect()
}

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
    pub mint: Pubkey,
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
        let mint = Pubkey::new_unique();

        Self {
            mollusk,
            admin_keypair,
            admin_address,
            test_fingerprint,
            mint,
        }
    }

    /// Initialize a campaign and return the campaign account data
    pub fn initialize_campaign(&mut self) -> InitializedCampaign {
        let (campaign_address, campaign_bump) = find_campaign_address(
            &self.admin_address, //
            &self.test_fingerprint,
        );

        let (initialize_campaign_ix, _, _) = build_initialize_campaign_ix(
            self.admin_address,
            campaign_address,
            self.test_fingerprint,
            self.mint,
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
            &[
                Check::success(), //
            ],
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
        }
    }

    /// Initialize a cohort with a real merkle tree and return the cohort data
    pub fn initialize_cohort_with_merkle_tree(
        &mut self,
        campaign: &InitializedCampaign,
        claimants: &[Pubkey],
        vaults: &[Pubkey],
        amount_per_entitlement: u64,
    ) -> InitializedCohort {
        // Create claimant entitlements pairs
        let claimant_entitlements: Vec<(Pubkey, u64)> = claimants
            .iter()
            .map(|&claimant| (claimant, amount_per_entitlement))
            .collect();

        // Create a real merkle tree using production function
        let merkle_tree = create_merkle_tree(&claimant_entitlements, vaults)
            .expect("Failed to create merkle tree");

        let merkle_root = merkle_tree.root().expect("Failed to get merkle root");

        // Derive cohort address
        let (cohort_address, cohort_bump) = find_cohort_v0_address(&campaign.address, &merkle_root);

        // Build cohort initialization instruction
        let (initialize_cohort_ix, _, _) = build_initialize_cohort_ix(
            self.admin_address,
            campaign.address,
            self.test_fingerprint,
            cohort_address,
            merkle_root,
            amount_per_entitlement,
            vaults.to_vec(),
        )
        .expect("Failed to build initialize_cohort instruction");

        println!(
            "Initializing cohort: {} (bump: {}, merkle_root: {:?}, vaults: {})",
            cohort_address,
            cohort_bump,
            merkle_root,
            vaults.len()
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
}

/// Result of campaign initialization
pub struct InitializedCampaign {
    pub address: Pubkey,
    pub bump: u8,
    pub admin_account: SolanaAccount,
    pub campaign_account: SolanaAccount,
}

/// Result of cohort initialization with merkle tree
pub struct InitializedCohort {
    pub address: Pubkey,
    pub bump: u8,
    pub merkle_tree: ClaimMerkleTree,
    pub cohort_account: SolanaAccount,
}

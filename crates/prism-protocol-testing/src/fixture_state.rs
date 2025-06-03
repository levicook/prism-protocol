use {
    crate::{deterministic_keypair, deterministic_pubkey, FixtureStage},
    prism_protocol_sdk::{
        compile_campaign, AddressFinder, CampaignCsvRow, CohortsCsvRow, CompiledCampaign,
    },
    rust_decimal::Decimal,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer as _,
};

pub struct FixtureState {
    pub address_finder: AddressFinder,

    pub admin_keypair: Keypair,
    pub mint_keypair: Keypair,

    pub compiled_campaign: CompiledCampaign,
    pub stage: FixtureStage,
}

impl Default for FixtureState {
    fn default() -> Self {
        let address_finder = AddressFinder::default();

        let admin_keypair = default_admin_keypair();
        let mint_keypair = default_mint_keypair();

        let campaign = compile_campaign(
            address_finder.clone(),
            &default_campaign_csv_rows(),
            &default_cohorts_csv_rows(),
            DEFAULT_BUDGET,
            mint_keypair.pubkey(),
            DEFAULT_MINT_DECIMALS,
            admin_keypair.pubkey(),
            DEFAULT_CLAIMANTS_PER_VAULT,
        )
        .expect("Failed to compile default campaign");

        Self {
            address_finder,
            admin_keypair,
            mint_keypair,
            compiled_campaign: campaign,
            stage: FixtureStage::default(),
        }
    }
}

pub const DEFAULT_BUDGET: Decimal = rust_decimal::dec!(1_000_000_000);

pub fn default_campaign_csv_rows() -> Vec<CampaignCsvRow> {
    vec![
        // ------------------------------------------------------------
        // EarlyAdopters
        // ------------------------------------------------------------
        CampaignCsvRow {
            cohort: "EarlyAdopters".to_string(),
            claimant: deterministic_pubkey("early_adopter_1"),
            entitlements: 1,
        },
        CampaignCsvRow {
            cohort: "EarlyAdopters".to_string(),
            claimant: deterministic_pubkey("early_adopter_2"),
            entitlements: 2,
        },
        // ------------------------------------------------------------
        // Investors
        // ------------------------------------------------------------
        CampaignCsvRow {
            cohort: "Investors".to_string(),
            claimant: deterministic_pubkey("investor_1"),
            entitlements: 1,
        },
        CampaignCsvRow {
            cohort: "Investors".to_string(),
            claimant: deterministic_pubkey("investor_2"),
            entitlements: 2,
        },
        // ------------------------------------------------------------
        // PowerUsers
        // ------------------------------------------------------------
        CampaignCsvRow {
            cohort: "PowerUsers".to_string(),
            claimant: deterministic_pubkey("power_user_1"),
            entitlements: 1,
        },
        CampaignCsvRow {
            cohort: "PowerUsers".to_string(),
            claimant: deterministic_pubkey("power_user_2"),
            entitlements: 2,
        },
        CampaignCsvRow {
            cohort: "PowerUsers".to_string(),
            claimant: deterministic_pubkey("power_user_3"),
            entitlements: 3,
        },
        // 🎯 NEW: Multi-cohort claimant - appears in both PowerUsers AND Team
        CampaignCsvRow {
            cohort: "PowerUsers".to_string(),
            claimant: deterministic_pubkey("multi_cohort_user"),
            entitlements: 5,
        },
        // ------------------------------------------------------------
        // Team
        // ------------------------------------------------------------
        CampaignCsvRow {
            cohort: "Team".to_string(),
            claimant: deterministic_pubkey("team_member_1"),
            entitlements: 1,
        },
        CampaignCsvRow {
            cohort: "Team".to_string(),
            claimant: deterministic_pubkey("team_member_2"),
            entitlements: 2,
        },
        CampaignCsvRow {
            cohort: "Team".to_string(),
            claimant: deterministic_pubkey("team_member_3"),
            entitlements: 3,
        },
        // 🎯 NEW: Same claimant appears in Team cohort with different entitlements
        CampaignCsvRow {
            cohort: "Team".to_string(),
            claimant: deterministic_pubkey("multi_cohort_user"),
            entitlements: 10,
        },
    ]
}

pub fn default_cohorts_csv_rows() -> Vec<CohortsCsvRow> {
    vec![
        CohortsCsvRow {
            cohort: "EarlyAdopters".to_string(),
            share_percentage: Decimal::from(5),
        },
        CohortsCsvRow {
            cohort: "Investors".to_string(),
            share_percentage: Decimal::from(10),
        },
        CohortsCsvRow {
            cohort: "PowerUsers".to_string(),
            share_percentage: Decimal::from(10),
        },
        CohortsCsvRow {
            cohort: "Team".to_string(),
            share_percentage: Decimal::from(75),
        },
    ]
}

pub fn default_mint_keypair() -> Keypair {
    deterministic_keypair("default_mint")
}

#[allow(unused)]
pub fn default_mint_pubkey() -> Pubkey {
    default_mint_keypair().pubkey()
}

pub const DEFAULT_MINT_DECIMALS: u8 = 9;

pub fn default_admin_keypair() -> Keypair {
    deterministic_keypair("default_admin")
}

#[allow(unused)]
pub fn default_admin_pubkey() -> Pubkey {
    default_admin_keypair().pubkey()
}

pub const DEFAULT_CLAIMANTS_PER_VAULT: usize = 10;

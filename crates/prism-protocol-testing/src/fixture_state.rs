use {
    crate::{deterministic_keypair, deterministic_pubkey, FixtureStage},
    prism_protocol_entities::compiled_cohorts,
    prism_protocol_sdk::{
        compile_campaign, AddressFinder, CampaignCsvRow, ClaimTreeType, CohortsCsvRow,
        CompiledCampaignDatabase, CompiledCampaignExt as _,
    },
    rust_decimal::{dec, Decimal},
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer as _,
};

pub struct FixtureState {
    admin_keypair: Keypair,
    mint_keypair: Keypair,

    pub ccdb: CompiledCampaignDatabase,

    pub stage: FixtureStage,
}

impl FixtureState {
    pub async fn new() -> Self {
        let admin_keypair = default_admin_keypair();
        let mint_keypair = default_mint_keypair();

        let compiled_campaign = compile_campaign(
            admin_keypair.pubkey(),
            DEFAULT_BUDGET,
            mint_keypair.pubkey(),
            DEFAULT_MINT_DECIMALS,
            &default_campaign_csv_rows(),
            &default_cohorts_csv_rows(),
            DEFAULT_CLAIMANTS_PER_VAULT,
            ClaimTreeType::V1,
        )
        .await
        .expect("Failed to compile default campaign");

        Self {
            admin_keypair,
            mint_keypair,
            ccdb: compiled_campaign,
            stage: FixtureStage::default(),
        }
    }

    pub fn address_finder(&self) -> &AddressFinder {
        &self.ccdb.address_finder
    }

    pub fn admin_address(&self) -> Pubkey {
        self.admin_keypair.pubkey()
    }

    pub fn admin_keypair(&self) -> &Keypair {
        &self.admin_keypair
    }

    pub fn campaign_address(&self) -> Pubkey {
        self.ccdb.address_finder.campaign
    }

    pub async fn campaign_budget_token(&self) -> u64 {
        self.ccdb.compiled_campaign().await.campaign_budget_token()
    }

    pub fn mint_address(&self) -> Pubkey {
        self.mint_keypair.pubkey()
    }

    pub async fn compiled_cohort_count(&self) -> u8 {
        self.ccdb.compiled_cohort_count().await
    }

    pub async fn compiled_cohorts(&self) -> Vec<compiled_cohorts::Model> {
        self.ccdb.compiled_cohorts().await
    }

    pub async fn mint_decimals(&self) -> u8 {
        let campaign = self.ccdb.compiled_campaign().await;
        campaign
            .mint_decimals
            .try_into()
            .expect("Mint decimals out of range")
    }

    pub fn mint_keypair(&self) -> &Keypair {
        &self.mint_keypair
    }

    pub fn prism_program_id(&self) -> Pubkey {
        self.ccdb.address_finder.prism_program_id
    }

    pub fn campaign_keypair(&self) -> &Keypair {
        self.ccdb
            .campaign_keypair
            .as_ref()
            .expect("Campaign keypair should exist")
    }
}

pub const DEFAULT_BUDGET: Decimal = dec!(1_000_000_000);

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

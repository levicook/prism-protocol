#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureStage {
    /// Campaign has been compiled
    CampaignCompiled,

    /// Campaign has been initialized but is inactive
    CampaignInitialized,

    /// Cohorts have been initialized and added to campaign
    CohortsInitialized,

    /// Vaults have been created for the cohort but are empty
    VaultsInitialized,

    /// Vaults have been funded
    VaultsFunded,

    /// The vault has been funded and activated
    VaultsActivated,

    /// The cohort has been activated
    CohortsActivated,

    /// The campaign has been activated and claims are allowed
    CampaignActivated,
}

impl FixtureStage {
    pub fn all() -> &'static [FixtureStage] {
        &[
            FixtureStage::CampaignCompiled,
            FixtureStage::CampaignInitialized,
            FixtureStage::CohortsInitialized,
            FixtureStage::VaultsInitialized,
            FixtureStage::VaultsFunded,
            FixtureStage::VaultsActivated,
            FixtureStage::CohortsActivated,
            FixtureStage::CampaignActivated,
        ]
    }

    /// Get the ordinal position of this stage in the progression
    pub fn ord(&self) -> u8 {
        match self {
            FixtureStage::CampaignCompiled => 0,
            FixtureStage::CampaignInitialized => 1,
            FixtureStage::CohortsInitialized => 2,
            FixtureStage::VaultsInitialized => 3,
            FixtureStage::VaultsFunded => 4,
            FixtureStage::VaultsActivated => 5,
            FixtureStage::CohortsActivated => 6,
            FixtureStage::CampaignActivated => 7,
        }
    }
}

impl PartialOrd for FixtureStage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FixtureStage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ord().cmp(&other.ord())
    }
}

impl Default for FixtureStage {
    fn default() -> Self {
        FixtureStage::CampaignCompiled
    }
}

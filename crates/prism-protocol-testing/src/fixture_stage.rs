#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureStage {
    /// Campaign has been initialized but is inactive
    CampaignInitialized,

    /// A cohort has been initialized and added to campaign
    CohortInitialized,

    /// A vault has been created for the cohort but is empty
    VaultInitialized,

    /// The vault has been funded and activated
    VaultActivated,

    /// The cohort has been activated
    CohortActivated,

    /// The campaign has been activated and claims are allowed
    CampaignActivated,
}

impl FixtureStage {
    /// Get the ordinal position of this stage in the progression
    pub fn ord(&self) -> u8 {
        match self {
            FixtureStage::CampaignInitialized { .. } => 0,
            FixtureStage::CohortInitialized { .. } => 1,
            FixtureStage::VaultInitialized => 2,
            FixtureStage::VaultActivated { .. } => 3,
            FixtureStage::CohortActivated => 4,
            FixtureStage::CampaignActivated { .. } => 5,
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

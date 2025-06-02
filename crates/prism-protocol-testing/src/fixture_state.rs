use {crate::FixtureStage, solana_pubkey::Pubkey};

#[derive(Debug, Clone)]
pub struct FixtureState {
    pub stage: Option<FixtureStage>,

    pub mint: Option<Pubkey>,
    pub campaign_fingerprint: Option<[u8; 32]>,
    pub go_live_slot: Option<u64>,

    pub campaign: Option<Pubkey>,

    pub cohort: Option<Pubkey>,
    pub cohort_merkle_root: Option<[u8; 32]>,

    pub vault: Option<Pubkey>,
    pub vault_expected_balance: Option<u64>,
}

impl Default for FixtureState {
    fn default() -> Self {
        Self {
            stage: None,

            mint: None,
            campaign_fingerprint: None,

            campaign: None,
            go_live_slot: None,

            cohort: None,
            cohort_merkle_root: None,

            vault: None,
            vault_expected_balance: None,
        }
    }
}

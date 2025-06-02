use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum CampaignStatus {
    Inactive,          // Deployed but not activated
    Active,            // Live and accepting claims
    Paused,            // Temporarily halted (resumable if not unstoppable)
    PermanentlyHalted, // Cannot be resumed, tokens reclaimable
}

impl Default for CampaignStatus {
    fn default() -> Self {
        CampaignStatus::Inactive
    }
}

#[account] // seed [CAMPAIGN_V0_SEED_PREFIX, fingerprint]
#[derive(InitSpace)]
pub struct CampaignV0 {
    /// The admin (authority) pubkey for this campaign.
    pub admin: Pubkey,

    /// The mint pubkey for the tokens being distributed in this campaign.
    pub mint: Pubkey,

    /// A unique fingerprint for this campaign.
    /// This is used in the Campaign PDA seeds.
    pub fingerprint: [u8; 32],

    /// IPFS hash of the final campaign database (published during activation, final deployment record)
    pub campaign_db_ipfs_hash: [u8; 32],

    /// Expected number of cohorts for this campaign (set at campaign initialization)
    pub expected_cohort_count: u8,

    /// Number of cohorts that have been initialized (incremented during cohort init)
    pub initialized_cohort_count: u8,

    /// Number of cohorts that have been activated (incremented during cohort activation)
    pub activated_cohort_count: u8,

    /// Current status of the campaign
    pub status: CampaignStatus,

    /// Whether the campaign can be paused/halted (false) or is unstoppable (true)
    /// Default: false (can be stopped), can be permanently set to true
    pub unstoppable: bool,

    /// Slot when campaign should go live (claims allowed after this slot)
    pub go_live_slot: u64,

    /// Bump seed for the Campaign PDA.
    pub bump: u8,
}

#[account] // seed [COHORT_V0_SEED_PREFIX, campaign, merkle_root]
#[derive(InitSpace)]
pub struct CohortV0 {
    /// Pubkey of the parent Campaign account.
    pub campaign: Pubkey,

    /// The Merkle root hash for this cohort's distribution.
    /// This root is used in the Cohort PDA seeds.
    pub merkle_root: [u8; 32],

    /// The amount of `mint` tokens to be distributed per single unit of entitlement.
    pub amount_per_entitlement: u64,

    /// Expected number of vaults for this cohort (set at cohort initialization)
    pub expected_vault_count: u8,

    /// Number of vaults that have been initialized (incremented during vault creation)
    pub initialized_vault_count: u8,

    /// Number of vaults that have been activated (incremented during vault activation)
    pub activated_vault_count: u8,

    /// Bump seed for the Cohort PDA.
    pub bump: u8,
}

#[account] // seed [CLAIM_RECEIPT_V0_SEED_PREFIX, cohort, claimant]
#[derive(InitSpace)]
pub struct ClaimReceiptV0 {
    /// The claimant who received the tokens.
    pub claimant: Pubkey,

    /// Pubkey of the Cohort account this receipt belongs to.
    pub cohort: Pubkey,

    /// The specific vault from which the tokens were claimed, as per the Merkle proof.
    pub assigned_vault: Pubkey,

    /// Timestamp of when the claim was successfully processed.
    pub claimed_at_timestamp: i64,

    /// Bump seed for the ClaimReceipt PDA.
    pub bump: u8,
}

use anchor_lang::prelude::*;

/// Maximum number of vaults that can be associated with a single cohort.
/// This helps in account sizing. Adjust as per expected sharding needs.
pub const MAX_VAULTS_PER_COHORT: usize = 16; // Example: allows up to 16 vaults per cohort

#[account] // seed [CAMPAIGN_V0_SEED_PREFIX, fingerprint]
#[derive(InitSpace)]
pub struct CampaignV0 {
    /// The admin that can manage the campaign (e.g., pause, unpause).
    pub admin: Pubkey,

    /// The mint of the token being distributed in this campaign.
    pub mint: Pubkey,

    /// A unique identifier for this specific campaign instance, derived from the Merkle roots of all its constituent cohorts.
    /// This is used in the Campaign PDA seeds.
    pub fingerprint: [u8; 32],

    /// Whether the campaign is currently active and allowing claims.
    pub is_active: bool,

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

    /// List of token account pubkeys that serve as vaults for this cohort.
    /// The size of this Vec is determined at initialization and capped by MAX_VAULTS_PER_COHORT.
    #[max_len(MAX_VAULTS_PER_COHORT)]
    pub vaults: Vec<Pubkey>,

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

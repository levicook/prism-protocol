use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    // Basic validation errors
    #[msg("Invalid Merkle proof provided.")]
    InvalidMerkleProof,
    #[msg("The provided Merkle root does not match the one stored in the cohort.")]
    MerkleRootMismatch,
    #[msg("A calculation resulted in a numeric overflow.")]
    NumericOverflow,
    #[msg("Invalid parameter: entitlements must be greater than zero.")]
    InvalidEntitlements,

    // Authorization and access errors
    #[msg("Token account owner mismatch: account is not owned by the expected authority.")]
    TokenAccountOwnerMismatch,
    #[msg("Campaign admin mismatch: signer is not the campaign administrator.")]
    CampaignAdminMismatch,

    // Specific PDA/constraint validation errors
    #[msg("Campaign fingerprint mismatch: the provided fingerprint does not match the campaign account.")]
    CampaignFingerprintMismatch,
    #[msg("Cohort campaign mismatch: the cohort does not belong to the specified campaign.")]
    CohortCampaignMismatch,
    #[msg("Mint mismatch: the provided mint does not match the campaign's mint.")]
    MintMismatch,

    // Campaign lifecycle errors
    #[msg("The campaign is not currently active.")]
    CampaignNotActive,
    #[msg("Campaign is currently active and cannot be modified.")]
    CampaignIsActive,
    #[msg("Campaign has already been activated.")]
    CampaignAlreadyActivated,
    #[msg("Campaign is unstoppable: cannot pause, halt, or modify an unstoppable campaign.")]
    CampaignIsUnstoppable,
    #[msg("Campaign is not paused: cannot resume a campaign that is not paused.")]
    CampaignNotPaused,
    #[msg("Campaign is not permanently halted: can only reclaim tokens from permanently halted campaigns.")]
    CampaignNotPermanentlyHalted,
    #[msg("Invalid campaign status transition: the requested state change is not allowed.")]
    InvalidStatusTransition,
    #[msg("Go-live date not reached: claims are not allowed until the campaign's go-live slot.")]
    GoLiveDateNotReached,

    // Campaign setup/activation errors
    #[msg("Invalid IPFS hash: hash cannot be empty.")]
    InvalidIpfsHash,
    #[msg("Go-live slot is in the past: must be current or future slot.")]
    GoLiveSlotInPast,
    #[msg("No cohorts expected: campaign must expect at least one cohort.")]
    NoCohortsExpected,
    #[msg("Not all cohorts activated: active_cohort_count must equal expected_cohort_count for campaign activation.")]
    NotAllCohortsActivated,

    // Cohort and vault setup errors
    #[msg("No vaults expected: cohort must expect at least one vault.")]
    NoVaultsExpected,
    #[msg("Vault index out of bounds: index exceeds the expected vault count for this cohort.")]
    VaultIndexOutOfBounds,
    #[msg("The number of vaults specified exceeds the maximum allowed per cohort.")]
    TooManyVaults,
    #[msg("Vault not initialized: cannot activate vault that hasn't been initialized.")]
    VaultNotInitialized,
    #[msg("Incorrect vault funding: vault balance must match expected amount.")]
    IncorrectVaultFunding,
    #[msg("Not all vaults activated: all vaults in cohort must be activated before cohort activation.")]
    NotAllVaultsActivated,

    // Claiming errors
    #[msg("Vault index mismatch: the assigned vault index is out of bounds for this cohort.")]
    AssignedVaultIndexOutOfBounds,
}

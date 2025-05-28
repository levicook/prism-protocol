use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Custom error message")]
    CustomError,
    #[msg("Invalid Merkle proof provided.")]
    InvalidMerkleProof,
    #[msg("Tokens for this entitlement have already been claimed.")]
    AlreadyClaimed,
    #[msg("The assigned vault is not valid for this cohort.")]
    InvalidAssignedVault,
    #[msg("The campaign is not currently active.")]
    CampaignNotActive,
    #[msg("This cohort is not currently active.")]
    CohortNotActive,
    #[msg("The vault does not have enough tokens to fulfill this claim.")]
    InsufficientVaultBalance,
    #[msg("A calculation resulted in a numeric overflow.")]
    NumericOverflow,
    #[msg("The provided Merkle root does not match the one stored in the cohort.")]
    MerkleRootMismatch,
    #[msg("The claimant in the proof does not match the transaction signer.")]
    ClaimantMismatch,
    #[msg("The number of entitlements claimed exceeds the total available for this cohort.")]
    EntitlementsExceeded,
    #[msg("The number of vaults provided exceeds the maximum allowed.")]
    MaxVaultsExceeded,
    #[msg("String parameter is too long.")]
    StringTooLong,
    #[msg("Claim deadline has passed.")]
    ClaimDeadlinePassed,
    #[msg("Invalid authority for this action.")]
    InvalidAuthority,
    #[msg("Unauthorized access or mismatched authority.")]
    Unauthorized,
    #[msg("Seed constraint violation: provided seeds do not match expected PDA derivation.")]
    ConstraintSeedsMismatch, // TODO this is an absolute garbage error message. Replace every usage with an error that indicates which seed is mismatched.
    #[msg("At least one vault must be provided for a cohort.")]
    NoVaultsProvided,
    #[msg("The number of vaults specified exceeds the maximum allowed per cohort.")]
    TooManyVaults,
    #[msg("Invalid vault index: vaults must be created sequentially starting from 0.")]
    InvalidVaultIndex,
    #[msg("Vault at this index has already been created.")]
    VaultAlreadyExists,

    #[msg("Campaign is active.")]
    CampaignIsActive,
}

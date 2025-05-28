use anchor_lang::prelude::*;

/// Seed prefixes for PDA derivation
#[constant]
pub const CAMPAIGN_V0_SEED_PREFIX: &[u8] = b"campaign_v0";

#[constant]
pub const COHORT_V0_SEED_PREFIX: &[u8] = b"cohort_v0";

#[constant]
pub const CLAIM_RECEIPT_V0_SEED_PREFIX: &[u8] = b"claim_receipt_v0";

#[constant]
pub const VAULT_V0_SEED_PREFIX: &[u8] = b"vault_v0";

/// Maximum number of vaults that can be associated with a single cohort.
pub const MAX_VAULTS_PER_COHORT: usize = 16;

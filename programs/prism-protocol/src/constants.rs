use anchor_lang::prelude::*;

/// Seed prefixes for PDA derivation

#[constant]
pub const COHORT_V0_SEED_PREFIX: &[u8] = b"cohort_v0";

#[constant]
pub const CLAIM_RECEIPT_V0_SEED_PREFIX: &[u8] = b"claim_receipt_v0";

#[constant]
pub const VAULT_SEED_PREFIX: &[u8] = b"vault"; // SPL TokenAccount

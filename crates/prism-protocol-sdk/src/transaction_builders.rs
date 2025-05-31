/*!
# Transaction Builders

This module provides high-level transaction builders for Prism Protocol operations.
All functions follow the naming pattern `build_*_tx` and return unsigned transactions
that can be signed and sent by the caller.

## Design Philosophy

- **Unsigned Transactions**: Return Transaction objects that need to be signed
- **Composable**: Can be combined into multi-instruction transactions
- **Error Handling**: Comprehensive validation before transaction creation
- **RPC Independence**: Don't make RPC calls, caller provides necessary data

## Usage

```rust
use prism_protocol_sdk::{build_initialize_campaign_tx, AddressFinder};
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

// Build transaction
let admin = Keypair::new();
let recent_blockhash = Hash::default(); // Get from RPC
let mint = Pubkey::new_unique();
let fingerprint = [0u8; 32];

let tx = build_initialize_campaign_tx(
    &admin.pubkey(),
    &mint,
    &fingerprint,
    recent_blockhash,
).expect("Failed to build transaction");

// Sign and send (caller responsibility)
let signed_tx = Transaction::from(tx).sign(&[&admin], recent_blockhash);
```
*/

use crate::{
    build_create_vault_ix, build_initialize_campaign_ix, build_initialize_cohort_ix,
    build_set_campaign_active_status_ix, AddressFinder,
};
use solana_sdk::{
    hash::Hash, instruction::Instruction, message::Message, pubkey::Pubkey,
    transaction::Transaction,
};

/// Errors that can occur during transaction building
#[derive(Debug)]
pub enum TransactionBuilderError {
    InvalidInput(String),
    InstructionBuilder(String),
    TransactionConstruction(String),
}

impl std::fmt::Display for TransactionBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionBuilderError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            TransactionBuilderError::InstructionBuilder(msg) => {
                write!(f, "Instruction building failed: {}", msg)
            }
            TransactionBuilderError::TransactionConstruction(msg) => {
                write!(f, "Transaction construction failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for TransactionBuilderError {}

pub type TransactionBuilderResult<T> = Result<T, TransactionBuilderError>;

/// Build transaction to initialize a campaign
pub fn build_initialize_campaign_tx(
    admin: &Pubkey,
    mint: &Pubkey,
    fingerprint: &[u8; 32],
    recent_blockhash: Hash,
) -> TransactionBuilderResult<Transaction> {
    let address_finder = AddressFinder::default();
    let (campaign_address, _) = address_finder.find_campaign_v0_address(admin, fingerprint);

    let (initialize_ix, _, _) =
        build_initialize_campaign_ix(*admin, campaign_address, *fingerprint, *mint)
            .map_err(|e| TransactionBuilderError::InstructionBuilder(e.to_string()))?;

    let message = Message::new(&[initialize_ix], Some(admin));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.message.recent_blockhash = recent_blockhash;

    Ok(transaction)
}

/// Build transaction to initialize a cohort
pub fn build_initialize_cohort_tx(
    admin: &Pubkey,
    campaign_fingerprint: &[u8; 32],
    merkle_root: &[u8; 32],
    amount_per_entitlement: u64,
    vault_count: u8,
    recent_blockhash: Hash,
) -> TransactionBuilderResult<Transaction> {
    let address_finder = AddressFinder::default();
    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(admin, campaign_fingerprint);
    let (cohort_address, _) = address_finder.find_cohort_v0_address(&campaign_address, merkle_root);

    let (initialize_ix, _, _) = build_initialize_cohort_ix(
        *admin,
        campaign_address,
        *campaign_fingerprint,
        cohort_address,
        *merkle_root,
        amount_per_entitlement,
        vault_count,
    )
    .map_err(|e| TransactionBuilderError::InstructionBuilder(e.to_string()))?;

    let message = Message::new(&[initialize_ix], Some(admin));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.message.recent_blockhash = recent_blockhash;

    Ok(transaction)
}

/// Build transaction to create a vault
pub fn build_create_vault_tx(
    admin: &Pubkey,
    campaign_fingerprint: &[u8; 32],
    merkle_root: &[u8; 32],
    mint: &Pubkey,
    vault_index: u8,
    recent_blockhash: Hash,
) -> TransactionBuilderResult<Transaction> {
    let address_finder = AddressFinder::default();
    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(admin, campaign_fingerprint);
    let (cohort_address, _) = address_finder.find_cohort_v0_address(&campaign_address, merkle_root);
    let (vault_address, _) = address_finder.find_vault_v0_address(&cohort_address, vault_index);

    let (create_vault_ix, _, _) = build_create_vault_ix(
        *admin,
        campaign_address,
        cohort_address,
        *mint,
        vault_address,
        *campaign_fingerprint,
        *merkle_root,
        vault_index,
    )
    .map_err(|e| TransactionBuilderError::InstructionBuilder(e.to_string()))?;

    let message = Message::new(&[create_vault_ix], Some(admin));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.message.recent_blockhash = recent_blockhash;

    Ok(transaction)
}

/// Build transaction to set campaign active status  
pub fn build_set_campaign_active_tx(
    admin: &Pubkey,
    campaign_fingerprint: &[u8; 32],
    is_active: bool,
    recent_blockhash: Hash,
) -> TransactionBuilderResult<Transaction> {
    let address_finder = AddressFinder::default();
    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(admin, campaign_fingerprint);

    let (set_active_ix, _, _) = build_set_campaign_active_status_ix(
        *admin,
        campaign_address,
        *campaign_fingerprint,
        is_active,
    )
    .map_err(|e| TransactionBuilderError::InstructionBuilder(e.to_string()))?;

    let message = Message::new(&[set_active_ix], Some(admin));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.message.recent_blockhash = recent_blockhash;

    Ok(transaction)
}

/// Build multi-instruction transaction from instruction list
/// Useful for combining multiple operations into a single transaction
pub fn build_multi_instruction_tx(
    instructions: Vec<Instruction>,
    payer: &Pubkey,
    recent_blockhash: Hash,
) -> TransactionBuilderResult<Transaction> {
    if instructions.is_empty() {
        return Err(TransactionBuilderError::InvalidInput(
            "Cannot create transaction with no instructions".to_string(),
        ));
    }

    let message = Message::new(&instructions, Some(payer));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.message.recent_blockhash = recent_blockhash;

    Ok(transaction)
}

/// Build transaction for campaign deployment workflow
/// Combines campaign initialization with cohort and vault creation
pub fn build_campaign_deployment_tx(
    admin: &Pubkey,
    mint: &Pubkey,
    fingerprint: &[u8; 32],
    cohorts: &[(
        /* merkle_root */ [u8; 32],
        /* amount_per_entitlement */ u64,
        /* vault_count */ u8,
    )],
    recent_blockhash: Hash,
) -> TransactionBuilderResult<Transaction> {
    let address_finder = AddressFinder::default();
    let (campaign_address, _) = address_finder.find_campaign_v0_address(admin, fingerprint);

    let mut instructions = Vec::new();

    // Add campaign initialization
    let (campaign_ix, _, _) =
        build_initialize_campaign_ix(*admin, campaign_address, *fingerprint, *mint)
            .map_err(|e| TransactionBuilderError::InstructionBuilder(e.to_string()))?;
    instructions.push(campaign_ix);

    // Add cohort initializations
    for (merkle_root, amount_per_entitlement, vault_count) in cohorts {
        let (cohort_address, _) =
            address_finder.find_cohort_v0_address(&campaign_address, merkle_root);

        let (cohort_ix, _, _) = build_initialize_cohort_ix(
            *admin,
            campaign_address,
            *fingerprint,
            cohort_address,
            *merkle_root,
            *amount_per_entitlement,
            *vault_count,
        )
        .map_err(|e| TransactionBuilderError::InstructionBuilder(e.to_string()))?;
        instructions.push(cohort_ix);
    }

    build_multi_instruction_tx(instructions, admin, recent_blockhash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{signature::Keypair, signer::Signer};

    #[test]
    fn test_build_initialize_campaign_tx() {
        let admin = Keypair::new();
        let mint = Pubkey::new_unique();
        let fingerprint = [1u8; 32];
        let recent_blockhash = Hash::default();

        let tx =
            build_initialize_campaign_tx(&admin.pubkey(), &mint, &fingerprint, recent_blockhash)
                .unwrap();

        assert_eq!(tx.message.instructions.len(), 1);
        assert_eq!(tx.message.recent_blockhash, recent_blockhash);
        assert_eq!(tx.message.header.num_required_signatures, 1);
    }

    #[test]
    fn test_build_multi_instruction_tx_empty_fails() {
        let admin = Keypair::new();
        let recent_blockhash = Hash::default();

        let result = build_multi_instruction_tx(vec![], &admin.pubkey(), recent_blockhash);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no instructions"));
    }

    #[test]
    fn test_build_campaign_deployment_tx() {
        let admin = Keypair::new();
        let mint = Pubkey::new_unique();
        let fingerprint = [1u8; 32];
        let recent_blockhash = Hash::default();

        let cohorts = vec![([2u8; 32], 1000, 2), ([3u8; 32], 2000, 3)];

        let tx = build_campaign_deployment_tx(
            &admin.pubkey(),
            &mint,
            &fingerprint,
            &cohorts,
            recent_blockhash,
        )
        .unwrap();

        // Should have 1 campaign + 2 cohorts = 3 instructions
        assert_eq!(tx.message.instructions.len(), 3);
        assert_eq!(tx.message.recent_blockhash, recent_blockhash);
    }
}

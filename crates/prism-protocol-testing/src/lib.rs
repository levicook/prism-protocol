mod campaign_snapshot;
mod fixture_stage;
mod fixture_state;
mod test_fixture;

use {
    litesvm::{
        types::{FailedTransactionMetadata, TransactionResult},
        LiteSVM,
    },
    sha2::Digest,
    solana_instruction::error::InstructionError,
    solana_keypair::{keypair_from_seed, Keypair},
    solana_pubkey::Pubkey,
    solana_signer::Signer as _,
    solana_system_interface::instruction::create_account,
    solana_transaction::Transaction,
    solana_transaction_error::TransactionError,
    spl_token::{
        instruction::initialize_mint2, solana_program::program_pack::Pack as _, state::Mint,
    },
    std::env,
};

pub use campaign_snapshot::{AccountChange, CampaignSnapshot};
pub use fixture_stage::FixtureStage;
pub use fixture_state::FixtureState;
pub use test_fixture::TestFixture;

/// Assert that a transaction failed with a specific custom error code
pub fn demand_custom_error_code<T>(
    result: Result<T, FailedTransactionMetadata>,
    expected_error_code: u32,
    error_name: &str,
) {
    match result {
        Ok(_) => {
            panic!(
                "❌ Transaction should have failed with {} error",
                error_name
            );
        }
        Err(failed_meta) => match failed_meta.err {
            TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
                assert_eq!(code, expected_error_code, "Expected {} error", error_name);
                println!("✅ Confirmed {} error (code: {})", error_name, code);
            }
            _ => {
                panic!(
                    "Expected TransactionError::InstructionError with {}, got: {:?}",
                    error_name, failed_meta.err
                );
            }
        },
    }
}

/// Assert AccountNotInitialized error (works with any success type)
pub fn demand_account_not_initialized_error<T>(result: Result<T, FailedTransactionMetadata>) {
    const ACCOUNT_NOT_INITIALIZED: u32 = 3012;
    let mapped_result = result.map(|_| ());
    demand_custom_error_code(
        mapped_result,
        ACCOUNT_NOT_INITIALIZED,
        "AccountNotInitialized",
    );
}

/// Assert InvalidAccountData error from SPL Token program (works with any success type)
pub fn demand_invalid_account_data_error<T>(result: Result<T, FailedTransactionMetadata>) {
    let mapped_result = result.map(|_| ());
    match mapped_result {
        Ok(_) => {
            panic!("❌ Transaction should have failed with InvalidAccountData error");
        }
        Err(failed_meta) => match failed_meta.err {
            TransactionError::InstructionError(_, InstructionError::InvalidAccountData) => {
                println!("✅ Confirmed InvalidAccountData error from SPL Token program");
            }
            _ => {
                panic!(
                    "Expected TransactionError::InstructionError with InvalidAccountData, got: {:?}",
                    failed_meta.err
                );
            }
        },
    }
}

/// Assert a Prism Protocol specific error (adds Anchor offset automatically)
pub fn demand_prism_error<T>(
    result: Result<T, FailedTransactionMetadata>,
    prism_error_code: u32,
    error_name: &str,
) {
    const ANCHOR_ERROR_OFFSET: u32 = 6000;
    let expected_code = prism_error_code + ANCHOR_ERROR_OFFSET;
    demand_custom_error_code(result, expected_code, error_name);
}

pub fn deterministic_keypair(identifier: &str) -> Keypair {
    let seed = sha2::Sha256::digest(identifier.as_bytes());
    keypair_from_seed(&seed).expect("SHA256 output should always be valid seed")
}

pub fn deterministic_pubkey(identifier: &str) -> Pubkey {
    deterministic_keypair(identifier).pubkey()
}

/// Load the Prism Protocol program into LiteSVM
/// Note: build.rs ensures that the program is built before this is called
pub fn load_prism_protocol(svm: &mut LiteSVM, program_id: Pubkey) {
    svm.add_program(
        program_id,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/prism_protocol.so"
        )),
    );
}

pub fn create_mint(
    svm: &mut LiteSVM,
    fee_payer: &Keypair,
    mint_keypair: &Keypair,
    decimals: u8,
    token_program_id: Option<Pubkey>,
) -> TransactionResult {
    let mint_size = Mint::LEN;
    let token_program_id = token_program_id.unwrap_or(spl_token::ID);

    let ix1 = create_account(
        &fee_payer.pubkey(),
        &mint_keypair.pubkey(),
        svm.minimum_balance_for_rent_exemption(mint_size),
        mint_size as u64,
        &token_program_id,
    );

    let ix2 = initialize_mint2(
        &token_program_id,
        &mint_keypair.pubkey(),
        &fee_payer.pubkey(),       // mint authority
        Some(&fee_payer.pubkey()), // freeze authority
        decimals,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix1, ix2],
        Some(&fee_payer.pubkey()),
        &[fee_payer, mint_keypair],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
}

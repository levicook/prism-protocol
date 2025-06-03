mod fixture_stage;
mod fixture_state;
mod test_fixture;

use {
    litesvm::{types::TransactionResult, LiteSVM},
    sha2::Digest,
    solana_keypair::{keypair_from_seed, Keypair},
    solana_pubkey::Pubkey,
    solana_signer::Signer as _,
    solana_system_interface::instruction::create_account,
    solana_transaction::Transaction,
    spl_token::{
        instruction::initialize_mint2, solana_program::program_pack::Pack as _, state::Mint,
    },
    std::env,
};

pub use fixture_stage::FixtureStage;
pub use fixture_state::FixtureState;
pub use test_fixture::TestFixture;

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

mod fixture_stage;
mod fixture_state;
mod test_fixture;

use {litesvm::LiteSVM, solana_pubkey::Pubkey, std::env};

pub use fixture_stage::FixtureStage;
pub use fixture_state::FixtureState;
pub use test_fixture::TestFixture;

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

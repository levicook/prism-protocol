// use prism_protocol::error::ErrorCode as PrismError;
// use prism_protocol_sdk::build_initialize_vault_v0_ix;
// use prism_protocol_testing::{FixtureStage, TestFixture};
// use solana_instruction::error::InstructionError;
// use solana_transaction_error::TransactionError;

use prism_protocol_testing::TestFixture;

/// Test vault initialization with wrong mint → MintMismatch error
///
/// Should test:
/// - Set up campaign and cohort with original mint
/// - Create a different (wrong) mint
/// - Attempt to initialize vault with wrong mint
/// - Verify fails with MintMismatch error code
/// - Verify proper error handling and no state corruption
#[ignore]
#[test]
#[ignore] // TODO: Implement this test
fn test_mint_mismatch() {
    let mut _test = TestFixture::default();

    todo!("Implement this test");

    // // Set up campaign and cohort with the original mint
    // let state = test
    //     .jump_to(FixtureStage::CohortsInitialized)
    //     .expect("cohort initialization failed");

    // let campaign_fingerprint = state
    //     .campaign_fingerprint
    //     .expect("Campaign fingerprint should be set");

    // let cohort_merkle_root = state
    //     .cohort_merkle_root
    //     .expect("Cohort merkle root should be set");

    // // Create a different mint (this is the key - using wrong mint)
    // let (wrong_mint, _) = test.create_mint(9).expect("Failed to create wrong mint");

    // // Manually try to initialize vault with wrong mint
    // let (ix, _, _) = build_initialize_vault_v0_ix(
    //     &test.address_finder,
    //     test.admin,
    //     campaign_fingerprint,
    //     cohort_merkle_root,
    //     wrong_mint, // Wrong mint!
    //     0,          // vault_index
    // )
    // .expect("Failed to build initialize vault ix");

    // let result = test.send_instructions(&[ix]);

    // match result {
    //     Ok(_) => {
    //         panic!("❌ Vault initialization should have failed with wrong mint!");
    //     }
    //     Err(failed_meta) => {
    //         // Verify we got the expected MintMismatch error
    //         const EXPECTED_ERROR: u32 = PrismError::MintMismatch as u32 + 6000; // Anchor offset

    //         match failed_meta.err {
    //             TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
    //                 assert_eq!(code, EXPECTED_ERROR, "Expected MintMismatch error");
    //             }
    //             _ => {
    //                 panic!(
    //                     "Expected TransactionError::InstructionError with MintMismatch, got: {:?}",
    //                     failed_meta.err
    //                 );
    //             }
    //         }
    //     }
    // }
}

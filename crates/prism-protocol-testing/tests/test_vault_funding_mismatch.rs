use prism_protocol_testing::TestFixture;

/// Test vault activation with incorrect funding → IncorrectVaultFunding error
///
/// Should test:
/// - Initialize campaign, cohort, and vault properly
/// - Fund vault with WRONG amount (insufficient funding)
/// - Attempt to activate vault with incorrect balance
/// - Verify fails with IncorrectVaultFunding error code
/// - Ensure precise balance validation is working
#[test]
#[ignore] // TODO: Implement this test
fn test_vault_funding_mismatch() {
    let mut _test = TestFixture::default();

    todo!("Implement this test");

    /*
    use prism_protocol::error::ErrorCode as PrismError;
    use prism_protocol_testing::{FixtureStage, TestFixture};
    use solana_instruction::error::InstructionError;
    use solana_transaction_error::TransactionError;

    // Get to vault initialized stage (creates campaign, cohort, vault)
    let state = test
        .jump_to(FixtureStage::VaultsInitialized)
        .expect("vault initialization failed");

    // Get the vault and mint from the returned state
    let mint = state.mint.expect("Mint should be initialized");
    let vault = state.vault.expect("Vault should be initialized");
    let expected_balance = 10_000_000_000u64; // Same as next_expected_balance()

    // Fund the vault with the WRONG amount (half of what's expected)
    let wrong_amount = expected_balance / 2; // 5_000_000_000
    test.mint_to(mint, vault, wrong_amount)
        .expect("Failed to fund vault with wrong amount");

    println!(
        "💰 Funded vault with {} tokens (expected: {})",
        wrong_amount, expected_balance
    );

    // Now try to activate the vault - this should fail with IncorrectVaultFunding
    let result = test.step_to(FixtureStage::VaultsActivated);

    match result {
        Ok(_) => {
            panic!("❌ Vault activation should have failed due to funding mismatch!");
        }
        Err(failed_meta) => {
            println!(
                "✅ Vault activation correctly failed: {:?}",
                failed_meta.err
            );

            const ANCHOR_ERROR_OFFSET: u32 = 6000;

            const EXPECTED_ERROR: u32 =
                PrismError::IncorrectVaultFunding as u32 + ANCHOR_ERROR_OFFSET;

            match failed_meta.err {
                TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
                    if code == EXPECTED_ERROR {
                        println!("✅ Confirmed IncorrectVaultFunding error");
                    } else {
                        panic!("❌ Expected error code {}, got {}", EXPECTED_ERROR, code);
                    }
                }
                _ => {
                    panic!(
                        "❌ Expected TransactionError::InstructionError with Custom code, got: {:?}",
                        failed_meta.err
                    );
                }
            }
        }
    }

    println!("🎉 Vault funding mismatch test passed!");
        */
}

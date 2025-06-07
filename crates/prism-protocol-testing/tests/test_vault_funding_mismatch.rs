use litesvm::LiteSVM;
use prism_protocol::error::ErrorCode as PrismError;
use prism_protocol_testing::{FixtureStage, FixtureState, TestFixture};
use solana_instruction::error::InstructionError;
use solana_transaction_error::TransactionError;

/// Test vault activation with incorrect funding → IncorrectVaultFunding error
///
/// Should test:
/// - Initialize campaign, cohort, and vault properly
/// - Fund vault with WRONG amount (insufficient funding)
/// - Attempt to activate vault with incorrect balance
/// - Verify fails with IncorrectVaultFunding error code
/// - Ensure precise balance validation is working
#[ignore = "Needs TestFixture API fixes: state.mint/vault access and step_to error handling"]
#[tokio::test]
async fn test_vault_funding_mismatch() {
    todo!("This test has API mismatches - TestFixture doesn't expose mint/vault from jump_to() and step_to() doesn't return Result. Need to fix the TestFixture API to properly test vault funding validation.");
}

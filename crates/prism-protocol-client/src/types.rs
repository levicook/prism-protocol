/*!
# Client Data Types

Clean data structures for transaction management and simulation results.
*/

use solana_client::rpc_response::RpcSimulateTransactionResult;
use solana_sdk::signature::Signature;

/// Result of transaction operations
#[derive(Debug)]
pub enum TransactionResult {
    /// Transaction was only simulated (dry-run mode)
    Simulated(RpcSimulateTransactionResult),
    /// Transaction was executed successfully
    Executed(Signature),
}

/// Result of transaction simulation
#[derive(Debug)]
pub struct SimulationResult {
    /// Whether the simulation succeeded
    pub success: bool,
    /// Compute units consumed
    pub compute_units: Option<u64>,
    /// Error message if simulation failed
    pub error: Option<String>,
    /// Raw simulation result
    pub raw: RpcSimulateTransactionResult,
}

impl SimulationResult {
    pub fn from_rpc_result(result: RpcSimulateTransactionResult) -> Self {
        let success = result.err.is_none();
        let compute_units = result.units_consumed;
        let error = result.err.as_ref().map(|e| e.to_string());

        Self {
            success,
            compute_units,
            error,
            raw: result,
        }
    }
}

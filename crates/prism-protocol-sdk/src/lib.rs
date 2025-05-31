/*!
# Prism Protocol SDK

This crate provides the core SDK functionality for Prism Protocol, including:

- **Campaign Compilation**: Convert CSV data into deployable campaign databases
- **Budget Allocation**: Precise token distribution calculations with safety checks
- **Address Management**: Derive all protocol PDAs and addresses
- **Transaction Building**: Create deployment and funding transactions

## Key Modules

- `campaign_compiler`: Main compilation logic
- `budget_allocation`: Isolated budget→token math with thorough testing
- `transaction_builders`: On-chain transaction creation
- `address_finder`: PDA derivation and address management
*/

mod address_finder;
pub mod budget_allocation;
pub mod campaign_compiler;
mod instruction_builders;
pub mod transaction_builders;

// Re-export main types
pub use address_finder::AddressFinder;
pub use budget_allocation::{AllocationError, AllocationResult, BudgetAllocator};
pub use campaign_compiler::{compile_campaign, CompilerError, CompilerResult};
pub use instruction_builders::*;
pub use prism_protocol::state::*;
pub use transaction_builders::*;

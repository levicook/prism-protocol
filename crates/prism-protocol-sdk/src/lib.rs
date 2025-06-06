/*!
# Prism Protocol SDK

This crate provides the core SDK functionality for Prism Protocol, including:

- **Campaign Compilation**: Convert CSV data into deployable campaign databases
- **Budget Allocation**: Precise token distribution calculations with safety checks
- **Address Management**: Derive all protocol PDAs and addresses

## Key Modules

- `campaign_compiler`: Main compilation logic
- `budget_allocation`: Isolated budget→token math with thorough testing
- `address_finder`: PDA derivation and address management
*/

mod address_finder;
mod budget_allocation;
mod campaign_compiler;
mod campaign_database;
mod instruction_builders;

// Re-export main types
pub use address_finder::AddressFinder;
pub use budget_allocation::{
    AllocationError, AllocationResult, BudgetAllocator, CohortAllocation, VaultAllocation,
};
pub use campaign_compiler::*;
pub use campaign_database::*;
pub use instruction_builders::*;
pub use prism_protocol::state::*;
pub use prism_protocol::ClaimLeaf;

// Re-export csv types
pub use prism_protocol_csvs::{CampaignCsvRow, CohortsCsvRow};

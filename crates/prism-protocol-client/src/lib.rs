/*!
# Prism Protocol Client

This crate provides a unified client for all Prism Protocol RPC operations and SPL token management.

## Purpose

This crate eliminates the technical debt of scattered RPC client code throughout the CLI by providing:

- **Unified RPC Client**: Single, properly configured client with connection management
- **Protocol Operations**: Clean abstractions for campaign, cohort, vault, and receipt operations
- **SPL Token Management**: Safe token operations using `anchor_spl` types
- **Transaction Excellence**: Simulation, sending, proper error handling, and explorer links

## Architecture

The main `PrismProtocolClient` provides all operations needed by CLI commands and the future API server.
It replaces 19+ scattered database connections and 6+ duplicated RPC client setups with clean, reusable abstractions.

Uses `anchor_spl` types exclusively for SPL token operations per our architecture decisions.

## Usage

```rust
use prism_protocol_client::{PrismProtocolClient, ClientResult};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn example() -> ClientResult<()> {
    let client = PrismProtocolClient::new("https://api.devnet.solana.com".to_string())?;

    // Protocol operations (note: using v0 methods with proper versioning)
    let fingerprint = [0u8; 32]; // Example fingerprint
    let admin_pubkey = Pubkey::from_str("11111111111111111111111111111112").unwrap();
    let campaign = client.get_campaign_v0(&fingerprint, &admin_pubkey)?;

    // SPL token operations using anchor_spl types
    let wsol_mint = spl_token::native_mint::id();
    let mint = client.get_mint(&wsol_mint)?;
    if let Some(mint_account) = mint {
        println!("WSOL has {} decimals", mint_account.decimals);
    }

    Ok(())
}
```
*/

pub mod client;
pub mod errors;
pub mod types;

// Re-export main types for convenience
pub use client::PrismProtocolClient;
pub use errors::{ClientError, ClientResult};
pub use types::{SimulationResult, TransactionResult};

// Re-export anchor_spl types for external use (following architecture decisions)
pub use anchor_spl::token::{Mint, TokenAccount};

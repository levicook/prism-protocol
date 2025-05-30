# Architecture Decisions

This document memorializes key architectural decisions for the Prism Protocol codebase to prevent technical debt and maintain consistency.

## SPL Token Handling Standard

**Decision**: Use `anchor_spl` types exclusively for all SPL token operations.

**Context**: We identified inconsistent patterns across the codebase:
- Manual `spl_token::state::Mint::unpack()` operations
- Custom wrapper types (`MintInfo`, `TokenAccountInfo`)  
- Scattered serialization approaches

**Standard**:
```rust
// ✅ CORRECT: Use anchor_spl types
use anchor_spl::token::{Mint, TokenAccount};

// Access fields via Deref to underlying spl_token types
let mint = client.get_mint(&mint_pubkey)?;
if let Some(mint_account) = mint {
    println!("Decimals: {}", mint_account.decimals); // Direct field access
}

// ❌ AVOID: Manual unpacking
use spl_token::state::{Mint, Account as TokenAccount};
let mint_account = Mint::unpack(&account_data)?;

// ❌ AVOID: Custom wrapper types  
struct MintInfo { /* custom fields */ }
```

**Benefits**:
- Automatic serialization via `AccountDeserialize`
- Consistent error handling
- Anchor ecosystem alignment
- Built-in account validation
- `Deref` access to underlying `spl_token` fields

**Implementation Status**: ✅ Completed in `prism-protocol-client` crate

**Yellow Flags**:
- Any `::unpack()` calls
- Manual byte array parsing
- Custom SPL token wrapper types
- Raw `get_account_data()` + manual parsing

## Connection Management Standard

**Decision**: Use unified client abstractions, not scattered database connections.

**Standard**:
```rust
// ✅ CORRECT: Unified client
let client = PrismProtocolClient::new(rpc_url)?;

// ❌ AVOID: Scattered RPC clients
let rpc_client = RpcClient::new(rpc_url);
```

**Benefits**:
- Centralized connection pooling
- Consistent error handling
- Reusable across CLI and API server

## CSV Schema Authority

**Decision**: Use `prism-protocol-csvs` crate as the single source of truth for CSV schemas.

**Standard**:
```rust
// ✅ CORRECT: Use authoritative schemas
use prism_protocol_csvs::{CampaignRow, CohortsRow};

// ❌ AVOID: Ad-hoc CSV parsing
let mut reader = csv::Reader::from_reader(file);
```

**Benefits**:
- Prevents schema drift
- Type-safe validation
- Version management
- Cross-file consistency checks 
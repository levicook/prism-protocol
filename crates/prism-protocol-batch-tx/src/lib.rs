/*!
# Prism Protocol Batch Transaction Client

Efficient batch transaction sending for Solana with automatic retry logic, simulation,
and optimization. Based on patterns from Solana CLI for production reliability.

## Quick Start

```rust
use prism_protocol_batch_tx::{BatchTxClient, TxBatchConfig};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, signature::Keypair};
use std::sync::Arc;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
let payer = Keypair::new();
let client = BatchTxClient::new(rpc_client, payer);

let instructions: Vec<Instruction> = vec![/* your instructions */];

// Simple usage - handles batching, simulation, retry automatically
let signatures = client.send_instructions(instructions).await?;
println!("Sent {} transactions successfully", signatures.len());
# Ok(())
# }
```

## Custom Configuration

```rust
# use prism_protocol_batch_tx::{BatchTxClient, TxBatchConfig};
# use solana_client::nonblocking::rpc_client::RpcClient;
# use solana_sdk::signature::Keypair;
# use std::sync::Arc;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
let config = TxBatchConfig {
    max_parallel_sends: 8,
    simulate_before_send: true,
    max_retries: 5,
    ..Default::default()
};

let payer = Keypair::new();
let client = BatchTxClient::with_config(rpc_client, payer, config);
# Ok(())
# }
```

## Multi-Signer Support

```rust
# use prism_protocol_batch_tx::BatchTxClient;
# use solana_client::nonblocking::rpc_client::RpcClient;
# use solana_sdk::{instruction::Instruction, signature::Keypair};
# use std::sync::Arc;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
# let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
# let payer = Keypair::new();
# let client = BatchTxClient::new(rpc_client, payer);
# let instructions: Vec<Instruction> = vec![];

// For cases where you need a different signer
let other_payer = Keypair::new();
let signatures = client.send_instructions_with_payer(instructions, &other_payer).await?;
# Ok(())
# }
```
*/

mod client;
mod config;
mod error;

pub use client::{BatchTxClient, CostEstimate};
pub use config::TxBatchConfig;
pub use error::TxBatchError;

// Re-export key Solana types for convenience
pub use solana_client::nonblocking::rpc_client::RpcClient;
pub use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    transaction::Transaction,
};

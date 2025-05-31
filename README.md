# Prism Protocol: Efficient Token Distribution on Solana

**Production-ready token distribution protocol with precise financial math and minimal write contention.**

## Overview

Prism Protocol enables scalable, verifiable token distribution on Solana by solving write contention through:

- **Vault Distribution**: Spreads claimants across multiple token vaults to minimize write locks
- **Merkle Verification**: Cryptographic proof-based claiming with minimal on-chain state
- **Precise Financial Math**: Zero floating-point errors using `rust_decimal` for exact calculations
- **Immutable Campaigns**: Campaign fingerprints ensure verifiable, unchangeable distribution rules

## Quick Start

### Installation

```bash
git clone https://github.com/yourusername/prism-protocol.git
cd prism-protocol
cargo build --release
```

### Basic Usage

```bash
# 1. Generate test fixtures with real keypairs
cargo run -p prism-protocol-cli -- generate-fixtures \
  --campaign-name "My Campaign" \
  --claimant-count 1000 \
  --cohort-count 3 \
  --budget "10000.0"

# 2. Compile campaign from CSV to database
cargo run -p prism-protocol-cli -- compile-campaign \
  --campaign-csv-in test-artifacts/fixtures/my-campaign/campaign.csv \
  --cohorts-csv-in test-artifacts/fixtures/my-campaign/cohorts.csv \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --budget "10000.0" \
  --admin-keypair admin.json \
  --campaign-db-out campaign.db

# 3. Deploy campaign on-chain
cargo run -p prism-protocol-cli -- deploy-campaign \
  --campaign-db-in campaign.db \
  --admin-keypair admin.json \
  --rpc-url https://api.devnet.solana.com

# 4. Check campaign status
cargo run -p prism-protocol-cli -- campaign-status \
  --campaign-db-in campaign.db \
  --rpc-url https://api.devnet.solana.com

# 5. Claim tokens (claimants)
cargo run -p prism-protocol-cli -- claim-tokens \
  --campaign-db-in campaign.db \
  --claimant-keypair claimant.json \
  --rpc-url https://api.devnet.solana.com
```

## Key Features

### 🔒 **Financial Safety**
- **Precise decimal math** using `rust_decimal` eliminates floating-point errors
- **Mint-aware calculations** respect token decimal constraints (SOL: 9, USDC: 6, etc.)
- **Conservative allocation** rounds down to prevent over-allocation, tracks dust
- **Example**: 1M USDC campaign has $0 error (vs potential $1,000 floating-point error)

### ⚡ **Performance**
- **Minimal write contention** through vault distribution and claim receipts
- **Optimized account usage** reduces rent costs and simplifies state management
- **Batch operations** for efficient deployment and claiming
- **Scalable architecture** handles 100K+ claimants efficiently

### 🔍 **Verifiability**
- **Campaign fingerprints** cryptographically tie campaigns to exact distribution rules
- **Merkle proofs** enable transparent eligibility verification
- **Immutable on-chain state** prevents tampering with distribution parameters
- **Complete audit trail** with deployment signatures and claim receipts

### 🏗️ **Developer Experience**
- **Modular architecture** with clean separation between on-chain and off-chain components
- **Comprehensive CLI** for campaign operators
- **Reusable SDK** for integration into dApps and services
- **Extensive testing** with automated validation and performance benchmarks

## Architecture

### Core Components

- **`prism-protocol`** - Minimal on-chain program with essential smart contract logic
- **`prism-protocol-cli`** - Command-line tool for campaign management
- **`prism-protocol-sdk`** - Client-side utilities for building transactions
- **`prism-protocol-db`** - Campaign database management and querying
- **`prism-protocol-merkle`** - Merkle tree construction and proof generation

### Campaign Lifecycle

1. **Generate**: Create test fixtures or process real claimant lists
2. **Compile**: Convert CSV data to deployment-ready database with precise budget allocation
3. **Deploy**: Initialize campaign, cohorts, and vaults on-chain with automated funding
4. **Claim**: Claimants submit merkle proofs to receive tokens
5. **Monitor**: Track campaign status, claim progress, and vault balances

## CLI Commands

### Campaign Management
```bash
# Generate test fixtures with organized directory structure
generate-fixtures --campaign-name <NAME> --claimant-count <COUNT> --budget <AMOUNT>

# Compile CSV data to database with precise decimal math
compile-campaign --campaign-csv-in <CSV> --cohorts-csv-in <CSV> --mint <MINT> --budget <AMOUNT>

# Deploy campaign infrastructure on-chain
deploy-campaign --campaign-db-in <DB> --admin-keypair <JSON>

# Monitor campaign status and vault funding
campaign-status --campaign-db-in <DB>
```

### Claiming Operations
```bash
# Check claimant eligibility across all cohorts
check-eligibility --campaign-db-in <DB> --claimant <PUBKEY>

# Execute token claiming with automatic token account creation
claim-tokens --campaign-db-in <DB> --claimant-keypair <JSON>

# Query claim history and status
query-claims --campaign-db-in <DB> --claimant <PUBKEY>
```

### Administrative
```bash
# Pause/resume campaigns
pause-campaign <FINGERPRINT> --keypair <JSON>
resume-campaign <FINGERPRINT> --keypair <JSON>

# Reclaim unused tokens after campaign completion
reclaim-tokens <CAMPAIGN> <COHORT> --keypair <JSON>
```

## Testing

### Quick Development
```bash
make smoke-test    # Fast validation (30s)
make dev-test      # CLI integration tests (1-2 min)
make test-all      # Complete test suite
```

### Performance Benchmarks
```bash
make test-performance  # Benchmark with 1K-100K datasets
```

The test suite includes:
- **Unit tests** across all workspace crates
- **CLI integration tests** with real command execution
- **Anchor program tests** for on-chain functionality
- **Performance benchmarks** with memory profiling
- **End-to-end validation** from fixtures to claiming

## Development

### Building
```bash
cargo build --release
```

### Testing
```bash
cargo test
anchor test  # Requires Solana CLI and Anchor framework
```

### Documentation
```bash
cargo doc --open
```

## Configuration

### Network Configuration
The CLI supports multiple Solana networks:
- **Localnet**: `http://127.0.0.1:8899` (default)
- **Devnet**: `https://api.devnet.solana.com`
- **Mainnet**: `https://api.mainnet-beta.solana.com`

### Token Support
- **SOL** (9 decimals): Native Solana token
- **USDC** (6 decimals): USD Coin
- **Custom SPL tokens**: Any token mint with proper decimal configuration

## Examples

### Small Campaign (1K claimants)
```bash
# Generate fixtures
cargo run -p prism-protocol-cli -- generate-fixtures \
  --campaign-name "Small Test" \
  --claimant-count 1000 \
  --budget "1000.0"

# Compile and deploy
cargo run -p prism-protocol-cli -- compile-campaign \
  --campaign-csv-in test-artifacts/fixtures/small-test/campaign.csv \
  --cohorts-csv-in test-artifacts/fixtures/small-test/cohorts.csv \
  --mint So11111111111111111111111111111111111111112 \
  --budget "1000.0" \
  --admin-keypair admin.json \
  --campaign-db-out small-test.db

cargo run -p prism-protocol-cli -- deploy-campaign \
  --campaign-db-in small-test.db \
  --admin-keypair admin.json
```

### Large Campaign (100K claimants)
```bash
# Generate large dataset with exponential distribution
cargo run -p prism-protocol-cli -- generate-fixtures \
  --campaign-name "Large Airdrop" \
  --claimant-count 100000 \
  --distribution exponential \
  --cohort-count 5 \
  --budget "1000000.0"

# Use higher claimants-per-vault for efficiency
cargo run -p prism-protocol-cli -- compile-campaign \
  --campaign-csv-in test-artifacts/fixtures/large-airdrop/campaign.csv \
  --cohorts-csv-in test-artifacts/fixtures/large-airdrop/cohorts.csv \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --budget "1000000.0" \
  --claimants-per-vault 500000 \
  --admin-keypair admin.json \
  --campaign-db-out large-airdrop.db
```

## Contributing

1. **Fork and clone** the repository
2. **Create a feature branch** for your changes
3. **Run the test suite** with `make test-all`
4. **Submit a pull request** with clear description

For core team members working with encrypted secrets, see [`secrets/README.md`](secrets/README.md) for setup instructions.

## License

Prism Protocol is licensed under the GNU General Public License v3.0.

Copyright (C) 2025 levicook@gmail.com

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
The full license text can be found in the `LICENSE` file in the root directory of this source tree.

---

**Built with ❤️ for the Solana ecosystem**

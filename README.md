# Prism Protocol: Efficient Token Distribution

## 1. Purpose

Prism Protocol aims to solve the challenge of distributing tokens to a large audience for popular projects on Solana, focusing on minimizing write contention on token accounts and optimizing on-chain state. This ensures a smoother, more scalable, and cost-effective airdrop or reward distribution process within the Solana ecosystem.

**Core Philosophy: Verifiable & Immutable Distribution Instances**

Prism Protocol is built on the principle of strong on-chain immutability and verifiability. Each token distribution campaign instance deployed through Prism is cryptographically tied to its exact parameters, including the specific set of recipients and their entitlements for each cohort.

-   **Campaign Merkle Identifier:** The cornerstone of this immutability is the `campaign_fingerprint`. This unique identifier for a `Campaign` PDA is a cryptographic hash derived from the Merkle roots of *all* its constituent cohorts. 
-   **Immutable Record:** Once a `Campaign` PDA is initialized on-chain with its `campaign_fingerprint`, it represents a fixed and unalterable set of distribution rules for that specific token mint. The set of cohorts and their respective Merkle trees are locked in.
-   **No In-Place Modifications:** The Prism Protocol does not support in-place modification of active or past distribution parameters within an existing `Campaign` PDA. Any change—such as altering recipient lists, amounts, or adding new cohorts—requires generating new Merkle roots. This, in turn, results in a new `campaign_fingerprint` and thus the deployment of a *new, distinct* `Campaign` PDA instance on-chain. 

This approach ensures maximum transparency, auditability, and predictability for every deployed distribution.

**Key Problems Addressed:**

- **Scalability & Congestion:** While Solana offers high throughput, naive airdrop approaches (e.g., direct transfers to all recipients or simple claim-from-one-source models) can still contribute to localized network congestion or necessitate users paying higher priority fees during peak claim periods.
- **Write-Lock Contention:** The primary challenge on Solana for large-scale distributions is write-lock contention. If many users attempt to claim tokens simultaneously from a single (or few) program-owned accounts, or if the distribution logic frequently modifies a small set of shared accounts, transactions can fail due to account locking, leading to a poor user experience.
- **Account Rent & State Management:** Creating and managing potentially millions of individual on-chain accounts (e.g., for claim status or temporary escrows) for a large airdrop is inefficient, costly in terms of SOL rent, and adds complexity to state management on Solana.

**Benefits:**

- **Minimized Write Contention:** By using per-claimant PDAs for claim status and distributing the token holdings across well-structured vaults, the protocol drastically reduces write-lock contention on critical accounts.
- **Reduced Transaction Fees:** Fewer on-chain state modifications and more efficient account usage lead to lower overall transaction fees for the distributing project and potentially for claimants.
- **Optimized Account Usage:** The Merkle tree approach, combined with efficient on-chain state for claim status (e.g., a single PDA per claimant per distribution), minimizes the number of on-chain accounts required, significantly reducing rent costs and simplifying state.
- **Enhanced User Experience:** Claimers benefit from a more reliable and smoother process due to reduced transaction failures.
- **Fairness & Transparency:** Merkle trees provide a transparent and verifiable method for defining and confirming eligibility for token claims.

## 2. System Design

The Prism Protocol is designed with modularity and security in mind, facilitating efficient token distribution through a combination of off-chain data preparation and on-chain verification and settlement.

**Core Components:**

-   **Token Vaults:** Secure, on-chain SPL token accounts holding the tokens for distribution. These are funded by the campaign operator and delegated to the `Campaign` PDA for transfers during claims.
-   **Prism On-Chain Program:** The primary smart contract responsible for:
    -   Managing `Campaign` PDAs, each uniquely identified by its `campaign_fingerprint`.
    -   Managing `Cohort` PDAs under each `Campaign`, each uniquely identified by the parent `Campaign`'s key and the `Cohort`'s own `merkle_root`.
    -   Verifying Merkle proofs submitted by claimants against the appropriate `Cohort`'s `merkle_root`.
    -   Authorizing token transfers from the designated `Token Vaults` to eligible claimants.
    -   Recording `ClaimReceipt` PDAs to prevent duplicate claims.
-   **Prism CLI (`prism-cli`):** An off-chain utility that campaign operators use to:
    -   Process claimant lists for each cohort.
    -   Generate a Merkle tree (and its `merkle_root`) for each cohort.
    -   Deterministically calculate the overall `campaign_fingerprint` from all cohort Merkle roots.
    -   Output all parameters needed for on-chain `Campaign` and `Cohort` initialization.
    -   Generate Merkle proofs for each claimant for frontend/dApp use.

### Modular Crate Architecture

Prism Protocol is organized into separate, focused crates for better maintainability and reusability:

**Core Crates:**
- **`prism-protocol`** - The minimal on-chain program containing only essential smart contract logic
- **`prism-protocol-sdk`** - Client-side utilities for building transactions and deriving addresses
- **`prism-protocol-merkle`** - Off-chain Merkle tree construction, proof generation, and verification utilities
- **`prism-protocol-testing`** - Shared testing utilities and fixtures for comprehensive test coverage

**Applications:**
- **`prism-cli`** - Command-line tool for campaign operators (planned implementation)

This modular design ensures:
- **Clean separation of concerns** between on-chain and off-chain functionality
- **Minimal on-chain program size** for efficient deployment and execution
- **Reusable components** that can be integrated into various client applications
- **Comprehensive testing infrastructure** shared across all components

### CLI Tool (`prism-protocol-cli`)

The Prism Protocol CLI provides campaign operators with powerful tools for managing token distributions at scale.

#### Installation & Usage

```bash
# Build the CLI
cargo build --release -p prism-protocol-cli

# Run commands
cargo run -p prism-protocol-cli -- <COMMAND>
```

#### Available Commands

**Generate Test Fixtures (Phase 0 - Available Now)**
```bash
# Generate 1,000 test claimants with realistic distribution across 3 cohorts
cargo run -p prism-protocol-cli -- generate-fixtures \
  --count 1000 \
  --distribution realistic \
  --cohort-count 3 \
  --campaign-csv-out campaign.csv \
  --cohorts-csv-out cohorts.csv

# Generate 1M claimants for benchmarking (deterministic, no real keypairs)
cargo run -p prism-protocol-cli -- generate-fixtures \
  --count 1000000 \
  --seed 42 \
  --distribution exponential \
  --min-entitlements 1 \
  --max-entitlements 1000 \
  --cohort-count 5 \
  --campaign-csv-out million-campaign.csv \
  --cohorts-csv-out million-cohorts.csv

# Generate 10M claimants for stress testing
cargo run -p prism-protocol-cli -- generate-fixtures \
  --count 10000000 \
  --distribution uniform \
  --cohort-count 10 \
  --campaign-csv-out stress-campaign.csv \
  --cohorts-csv-out stress-cohorts.csv
```

**Compile Campaign from CSV Files (Phase 1 - Available Now)**
```bash
# Compile campaign from CSV files
cargo run -p prism-protocol-cli -- compile-campaign \
  --campaign-csv-in campaign.csv \
  --cohorts-csv-in cohorts.csv \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --admin-keypair ~/.config/solana/id.json \
  --campaign-db-out campaign.db
```

**Campaign Management (Planned - Phase 2+)**
```bash
# Deploy campaign on-chain
prism-protocol deploy-campaign --config campaign-config.yaml --admin-keypair admin.json

# Deploy individual cohorts
prism-protocol deploy-cohort --campaign <fingerprint> --merkle-root <root> --admin-keypair admin.json

# Administrative operations
prism-protocol pause-campaign <campaign-fingerprint> --admin-keypair admin.json
prism-protocol resume-campaign <campaign-fingerprint> --admin-keypair admin.json
prism-protocol reclaim-tokens <campaign> <cohort> --admin-keypair admin.json

# Status monitoring
prism-protocol campaign-status <campaign-fingerprint>
```

#### Fixture Generation Features

- **Deterministic Generation**: Same seed produces identical results for reproducible benchmarks
- **Multiple Distributions**: 
  - `uniform` - Even distribution across entitlement range
  - `realistic` - Weighted towards lower values (more realistic user behavior)
  - `exponential` - Exponential decay distribution
- **Multi-Cohort Support**: Generates both campaign.csv and cohorts.csv files with configurable cohort counts
- **Scalable**: Efficiently generates millions of test claimants without real keypairs
- **Progress Tracking**: Built-in progress indicators for large datasets
- **CSV Output**: Standard format compatible with campaign generation tools

#### Campaign Generation Features

- **CSV Input Processing**: Reads campaign claimants and cohort configuration from CSV files
- **Keypair Validation**: Uses Solana SDK to properly read and validate admin keypairs
- **SQLite Database Output**: Creates comprehensive campaign database with:
  - Campaign metadata (fingerprint, mint, admin, timestamps)
  - Cohort details (merkle roots, token requirements, vault counts)
  - Claimant records (entitlements, vault assignments, merkle proofs)
  - Vault funding requirements and claimant distribution
- **Vault Count Calculation**: Automatically determines optimal vault distribution based on claimant counts
- **Data Integrity**: Validates cohort consistency between input files

## Testing & Development

Prism Protocol includes a comprehensive test automation system for reliable development and validation.

### Test Automation System

The project uses a multi-layered testing approach with automated test orchestration:

```bash
# Quick development feedback
make smoke-test     # Fast smoke tests (30 seconds)
make dev-test       # Clean + CLI tests (1 minute)

# Comprehensive testing
make test-unit      # All unit tests across workspace
make test-cli       # CLI integration tests with real command execution
make test-anchor    # Anchor on-chain program tests
make test-integration # Full integration tests with Mollusk SVM
make test-all       # Everything (unit + CLI + anchor + integration)

# Performance analysis
make test-performance # Benchmarks with 1K-100K datasets

# Utilities
make clean-test     # Clean all test artifacts
make help          # Show all available test commands
```

### CLI Integration Testing

The CLI test suite (`scripts/test-cli.sh`) provides comprehensive validation:

**Features:**
- ✅ **Real CLI execution** - Actually runs `cargo run -p prism-protocol-cli` commands
- ✅ **Comprehensive assertions** - File existence, content validation, database checks
- ✅ **Error handling tests** - Ensures commands fail appropriately with bad inputs
- ✅ **Deterministic behavior** - Validates same seed produces identical results
- ✅ **Database validation** - Uses `sqlite3` to verify database structure and content
- ✅ **Automatic cleanup** - No test artifacts left behind

**Test Coverage:**
```bash
# Tests all these scenarios:
- CLI help commands work correctly
- Fixture generation (multiple distributions, sizes, cohorts)
- Campaign compilation (CSV → SQLite database with merkle trees)
- Error handling (missing files, invalid inputs)
- Deterministic behavior (same seed = same output)
- Database content validation (table structure, record counts, merkle data)
```

### Performance Testing

The performance test suite (`scripts/test-performance.sh`) provides benchmarking:

- **Fixture generation**: 1K → 100K claimants with throughput measurements
- **Campaign compilation**: 1K → 10K claimants with timing and memory usage
- **Database size analysis**: Tracks storage requirements as datasets scale
- **Memory profiling**: Uses GNU `time` for detailed memory usage analysis
- **Performance reports**: Generates detailed reports with optimization recommendations

### Test Artifacts Management

All test files are automatically organized and cleaned up:

```
test-artifacts/
├── cli-tests/           # CLI integration test artifacts
│   ├── test-admin.json  # Test keypair for CLI tests
│   ├── *.csv           # Generated fixture files
│   ├── *.db            # Compiled campaign databases
│   └── test-*.csv      # Test-specific fixture files
└── performance-tests/   # Performance benchmark artifacts
    ├── test-admin.json  # Test keypair for performance tests
    ├── perf-*.csv      # Performance test fixtures
    ├── perf-*.db       # Performance test databases
    └── performance-report.txt # Performance analysis reports
```

**Automatic Management:**
- **Created by**: Test scripts automatically create subdirectories as needed
- **Cleaned by**: `make clean-test` removes the entire directory
- **Git ignored**: All contents are ignored by git (see `.gitignore`)

### Development Workflow

**Quick Development Cycle:**
```bash
# Make changes to CLI code
make smoke-test     # Quick validation (30s)
make dev-test       # Full CLI test cycle (1-2 min)
```

**Pre-commit Validation:**
```bash
make test-all       # Complete test suite
```

**Performance Baseline:**
```bash
make test-performance  # Establish performance benchmarks
```

### Test Dependencies

The test system automatically handles dependencies but requires:
- **Solana CLI** - For keypair generation and validation
- **SQLite3** - For database content validation (optional, tests skip if unavailable)
- **bc** - For performance calculations (auto-installed on supported systems)

**Key Processes:**

1.  **Setup & Funding (Operator using `prism-cli`):**
    -   The operator defines campaign parameters (e.g., a descriptive name for off-chain use, the SPL token mint) and details for each cohort (claimant lists, reward per entitlement) in a configuration file.
    -   The `prism-cli` processes this configuration:
        1.  For each defined cohort, it generates a list of `ClaimLeaf` data (claimant, assigned vault, entitlements) and computes its unique `merkle_root`.
        2.  After all cohort Merkle roots are determined, the CLI sorts these roots, concatenates them, and hashes the result to produce the single `campaign_fingerprint`.
    -   The CLI outputs the `campaign_fingerprint`, individual cohort `merkle_root`s, and other data needed for on-chain transactions.
    -   The operator funds the necessary Token Vaults and delegates their authority to the (future) `Campaign` PDA (whose address can be pre-calculated from the `campaign_fingerprint`).
    -   The operator submits transactions to initialize the `Campaign` PDA (using `campaign_fingerprint`) and then each `Cohort` PDA (using the `Campaign` PDA's key and the cohort's `merkle_root`).

2.  **Claiming Process (User via dApp):**
    -   A Claimant connects to a dApp integrated with Prism.
    -   The dApp, using the `campaign_fingerprint` and the claimant's public key, retrieves the claimant's specific `merkle_proof`, `assigned_vault`, `entitlements`, and the relevant `cohort_merkle_root` from data provided by the campaign operator (generated by `prism-cli`).
    -   The Claimant submits a `claim_reward` transaction including these details.
    -   The Prism on-chain program:
        -   Derives and verifies the `Campaign` PDA using `campaign_fingerprint`.
        -   Derives and verifies the `Cohort` PDA using the `Campaign` key and `cohort_merkle_root`.
        -   Verifies the Merkle proof against the `Cohort`'s `merkle_root`.
        -   Initializes a `ClaimReceipt` PDA for the claimant.
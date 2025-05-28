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

**Campaign Generation (Phase 1 - Available Now)**
```bash
# Generate campaign database from CSV files
cargo run -p prism-protocol-cli -- generate-campaign \
  --campaign-csv-in campaign.csv \
  --cohorts-csv-in cohorts.csv \
  --mint So11111111111111111111111111111111111111112 \
  --admin-keypair admin.json \
  --claimants-per-vault 200000 \
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
        -   Initializes a `ClaimReceipt` PDA to prevent replays.
        -   If valid, transfers tokens from the `assigned_vault` (owned by the `Campaign` PDA) to the claimant.

**Handling Common Operational Scenarios:**

Due to the immutable nature of on-chain `Campaign` instances:

-   **Adding a new distribution/cohort to a logical campaign (e.g., "Wave 2")?**
    -   You define this new cohort. The `prism-cli` generates its Merkle root. This changes the overall set of cohort roots, so the CLI calculates a *new* `campaign_fingerprint`. A new `Campaign` PDA must be initialized on-chain. Users will interact with this new `Campaign` PDA for claims from this new cohort.
-   **Modifying an existing cohort (e.g., correcting amounts, adding users)?**
    -   This requires generating a new Merkle root for that cohort. This, too, results in a new `campaign_fingerprint` and requires initializing a new `Campaign` PDA instance.

**Administrative Operations:**

-   **Pausing/Unpausing a Campaign Instance:** The campaign authority can call an instruction (`set_campaign_active_status`) to toggle the `is_active` flag on a specific `Campaign` PDA (identified by its `campaign_fingerprint`), effectively pausing or unpausing claims for all its cohorts.
-   **Withdrawing Unclaimed Funds:** After a distribution period, the campaign authority can initiate withdrawal of remaining tokens from the vaults associated with a campaign instance. (Specific instruction TBD, e.g., `withdraw_from_vault`).

**Security Considerations:**

- **Proof Verification:** Robust Merkle proof verification is central to the system.
- **Replay Prevention:** The contract must ensure that each valid leaf in a Merkle tree can only be claimed once.
- **Ownership & Access Control:**
  - Only authorized administrators (e.g., the Distributor) can register new Merkle roots or manage campaign parameters.
  - Mechanisms for pausing/unpausing claims, or for recovering unclaimed tokens after a distribution period ends, will be access-controlled.
- **Data Availability:** While proofs are verified on-chain, the full Merkle tree data (and individual proofs) must be made available off-chain by the Distributor. The integrity of this off-chain data is crucial.

## 3. General Implementation

This section outlines the proposed technical implementation details for the Prism Protocol.

**Technology Stack (Illustrative - e.g., Solana Ecosystem):**

- **Blockchain:** Solana (chosen for its high throughput and low transaction fees, ideal for minimizing contention).
- **Smart Contract Language:** Rust (using the Anchor framework for rapid development and security).
- **Off-Chain Services:** Node.js/TypeScript or Python for Merkle tree generation, proof serving API, and potentially a reference UI.
- **Client-Side Libraries:** JavaScript/TypeScript for easy integration into dApp frontends.

**On-Chain Program (Prism Protocol):**

The core on-chain program will manage distributions.

- **Key Accounts & State (Simplified Overview):**
  - `Campaign` Account (PDA seeded by `[b"campaign", campaign_fingerprint.as_ref()]`):
    - `authority`: Pubkey of the campaign administrator.
    - `mint`: Pubkey of the SPL token being distributed.
    - `campaign_fingerprint`: The `[u8; 32]` hash derived from all its cohort roots.
    - `is_active`: Boolean flag to pause/unpause claims for this entire campaign instance.
    - `bump`: PDA bump seed.
  - `Cohort` Account (PDA seeded by `[b"cohort", campaign_pda.key().as_ref(), cohort_merkle_root.as_ref()]`):
    - `campaign`: Pubkey of the parent `Campaign` PDA.
    - `merkle_root`: The `[u8; 32]` Merkle root for this specific cohort's distribution.
    - `reward_per_entitlement`: u64 amount per entitlement.
    - `vaults`: `Vec<Pubkey>` of SPL Token Accounts holding tokens for this cohort, delegated to the `Campaign` PDA.
    - `bump`: PDA bump seed.
  - `ClaimReceipt` Account (PDA seeded by `[b"claim_receipt", cohort_pda.key().as_ref(), claimant.key().as_ref()]`):
    - Stores details of a claim to prevent replays.

- **Key Instructions (Functions - Simplified Overview):**
  - `initialize_campaign(ctx, campaign_fingerprint, mint)`: Creates `Campaign` PDA. Admin only.
  - `initialize_cohort(ctx, campaign_fingerprint, cohort_merkle_root, reward_per_entitlement, vaults)`: Creates `Cohort` PDA. Admin only.
  - `claim_reward(ctx, campaign_fingerprint, cohort_merkle_root, merkle_proof, assigned_vault, entitlements)`: Allows users to claim tokens.
  - `set_campaign_active_status(ctx, campaign_fingerprint, is_active)`: Admin toggles campaign activity.
  - *(Future)* `withdraw_unclaimed_funds(...)`: Admin recovers funds.

**Developer Tutorials & SDK:**

- **Tutorials:**
  - Setting up a new distribution campaign instance (using `prism-cli` to generate cohort roots, the `campaign_fingerprint`, funding vaults, deploying on-chain).
- **Client SDK (JavaScript/TypeScript):**
  - `getClaimantInfo(apiUrl, campaignId, claimantAddress)`: Fetches amount and proof from the Distributor's hosted service.
  - `buildClaimTransaction(program, distributionConfigPubkey, claimantPubkey, amount, proof)`: Constructs the Solana transaction for the `claim_tokens` instruction.
  - Helper functions for Merkle tree generation and verification (can be used by Distributors or for testing).

**Future Considerations:**

- **Vesting:** Integrate with existing Solana vesting contracts or add vesting logic directly.
- **Multi-Token Vaults:** Allow a single `DistributionConfig` to pull from different vaults if needed (adds complexity).
- **Batch Claims:** For users eligible in multiple small distributions, explore ways to batch claims.
- **Fee-payer abstraction (e.g., sponsored transactions).**
- **NFT Airdrops:** Adapt the leaf structure and claim logic to support SPL Non-Fungible Tokens.

## 4. License

Prism Protocol is licensed under the GNU General Public License v3.0.

Copyright (C) 2025 levicook@gmail.com

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
The full license text can be found in the `LICENSE` file in the root directory of this source tree.

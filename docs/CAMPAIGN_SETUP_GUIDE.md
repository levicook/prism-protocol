# Prism Protocol: Campaign Setup Guide for Operators (Highly Automated)

> **📋 IMPLEMENTATION STATUS:**
>
> - **✅ Phase 1 (Campaign Compilation): COMPLETED** - The `compile-campaign` command is fully implemented with complete merkle tree integration, campaign fingerprint calculation, and SQLite database output.
> - **🚧 Phase 2 (On-Chain Deployment): IN PROGRESS** - Transaction building and deployment commands are planned for future development.
> - **📋 Phase 3+ (Campaign Management): PLANNED** - Administrative operations for live campaigns.

This guide walks you through the process of setting up and launching a token distribution campaign using the Prism Protocol. The CLI tool handles campaign compilation and will eventually support full deployment automation.

**Core Principle: On-Chain Immutability via Merkle Identifiers**

Prism Protocol enforces strong immutability for its on-chain records. A Campaign and its Cohorts are uniquely identified by cryptographic hashes, ensuring that a specific on-chain Campaign PDA represents a fixed and unalterable set of distribution rules.

- **Cohort Immutability:** Each `Cohort` PDA is uniquely identified by its parent Campaign and its specific Merkle distribution root (`merkle_root`). This root is generated from a list of claimants and their entitlements for that cohort. Once a `Cohort` is initialized on-chain with its `merkle_root`, its distribution logic is immutable.

- **Campaign Immutability (`campaign_fingerprint`):** A `Campaign` PDA is uniquely identified on-chain by a `campaign_fingerprint`. This identifier is a cryptographic hash derived from the Merkle roots of _all_ its constituent Cohorts. This means a `Campaign` PDA inherently represents a specific, immutable collection of Cohort distributions.

  - **Generation:** The `prism-protocol-cli` will first calculate the `merkle_root` for every cohort you define. Then, it will deterministically combine these cohort Merkle roots (e.g., by sorting them, concatenating, and hashing) to produce the single `campaign_fingerprint`.
  - This `campaign_fingerprint` is then used as a seed to create the unique `Campaign` PDA.

- **Handling Changes/Updates/New Waves:**
  - If you need to modify a specific cohort's distribution (e.g., add new claimants, change rewards), you will generate a _new_ `merkle_root` for that cohort, leading to a _new, distinct_ `Cohort` PDA for that updated distribution.
  - If you want to add a new cohort to what you logically consider an ongoing campaign, or if any existing cohort's Merkle root changes, the overall set of cohort Merkle roots changes. This will result in a _new_ `campaign_fingerprint` being calculated by the CLI. Consequently, you will initialize a _new, distinct_ `Campaign` PDA. The original `Campaign` PDA and its associated Cohorts remain untouched and immutable.
  - Think of each `Campaign` PDA as a complete, versioned snapshot of a set of distributions.

This model ensures maximum transparency and verifiability. The primary on-chain control after launch is the `is_active` flag on a `Campaign` PDA (acting as an on/off switch for all its cohorts) and the eventual ability to withdraw unclaimed funds.

## Phase 1: Campaign & Cohort Definition (CSV Files)

Your main focus is defining the campaign's claimants and cohort parameters using CSV files that the CLI can process.

1.  **Prepare Campaign Claimants CSV:**

    - Create a CSV file with claimant information:
      - `cohort`: The cohort name this claimant belongs to
      - `claimant`: The Solana public key of each claimant
      - `entitlements`: Number of entitlements this claimant can claim

2.  **Prepare Cohorts Configuration CSV:**

    - Create a CSV file defining each cohort:
      - `cohort`: Unique cohort identifier (matches campaign CSV)
      - `amount_per_entitlement`: Token amount per entitlement (in base units)

3.  **Prepare Admin Keypair:**
    - Ensure you have the authority keypair that will control the campaign
    - This keypair will pay for transactions and be set as the campaign authority

## Phase 2: Execute CLI Campaign Compilation – Generate Merkle Trees, Campaign Fingerprint, & Database

The `compile-campaign` command processes your campaign configuration and generates all necessary data structures for deployment.

1.  **Prepare Your Input Files:**

    - **Campaign CSV (`campaign.csv`):** Contains claimant information with columns:

      ```csv
      cohort,claimant,entitlements
      early_contributors,7BgBvyjrZX8YKHGoW9Y8929nsq6TsQANzvsGVEpVLUD8,5
      community_mvps,9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM,2
      ```

    - **Cohorts CSV (`cohorts.csv`):** Defines cohort parameters:
      ```csv
      cohort,amount_per_entitlement
      early_contributors,1000000000
      community_mvps,250000000
      ```

2.  **Run Campaign Compilation:**

    ```bash
    cargo run -p prism-protocol-cli -- compile-campaign \
        --campaign-csv campaign.csv \
        --cohorts-csv cohorts.csv \
        --mint YourSplTokenMintPubkey... \
        --admin-keypair /path/to/your_authority_keypair.json
    ```

3.  **CLI Actions (Fully Automated):**

    - Validates input CSV files and admin keypair
    - **For each cohort:**
      - Processes claimant lists and calculates entitlements
      - Assigns claimants to vaults using consistent hashing
      - Generates merkle tree and calculates `merkle_root`
      - Generates individual merkle proofs for all claimants
      - Calculates exact token funding requirements
    - **Calculates `campaign_fingerprint`:**
      - Collects all cohort `merkle_root`s
      - Sorts roots deterministically
      - Hashes sorted roots to produce unique campaign identifier
    - **Generates SQLite Database:** Contains complete campaign structure with:
      - Campaign metadata and fingerprint
      - Cohort definitions with merkle roots
      - Claimant records with proofs and vault assignments
      - Vault funding requirements

4.  **Review CLI Output:**
    - **Database File:** `campaign_<fingerprint>.db` with complete campaign data
    - **Campaign Fingerprint:** Unique identifier for on-chain deployment
    - **Funding Summary:** Exact token amounts needed per vault
    - **Validation Results:** Confirmation of data integrity and consistency

## Phase 3: On-Chain Deployment (Future Implementation)

> **🚧 IN PROGRESS:** The following deployment commands are planned but not yet implemented. For now, use the generated database and SDK utilities for manual deployment.

1.  **Fund Vaults & Delegate Authority:**

    - Use the funding requirements from the database to fund token vaults
    - Delegate authority to the campaign admin keypair

2.  **Deploy Campaign & Cohorts On-Chain:**
    - **Planned Commands:**

      ```bash
      # Deploy campaign
      cargo run -p prism-protocol-cli -- deploy-campaign \
          --database campaign_<fingerprint>.db \
          --admin-keypair /path/to/admin.json

      # Deploy individual cohorts
      cargo run -p prism-protocol-cli -- deploy-cohort \
          --database campaign_<fingerprint>.db \
          --cohort early_contributors \
          --admin-keypair /path/to/admin.json
      ```

    - **On-Chain Operations:**
      - Initialize `Campaign` PDA with fingerprint and mint
      - Initialize each `Cohort` PDA with merkle root and parameters
      - Validate vault funding and authority delegation

## Phase 4: Distribute Lookup Information & Go Live

1.  **Host Lookup Database:**

    - The generated SQLite database contains all necessary claimant information
    - Host the database or create an API that queries it for claimant data
    - Database contains: campaign fingerprint, cohort merkle roots, claimant proofs, vault assignments

2.  **Integrate Lookup Mechanism into dApp:**
    - The dApp needs the `campaign_fingerprint` of the target campaign
    - For a user, the dApp queries the database for:
      - Eligible cohorts and entitlements
      - Merkle proofs for each cohort
      - Assigned vault public keys
    - The dApp constructs `claim_tokens_v0` transactions with the retrieved data

## Phase 5: Post-Campaign Management (Future Implementation)

> **📋 PLANNED:** Administrative operations for live campaigns.

- Monitor campaign status and claim activity
- Pause/resume campaigns as needed
- Withdraw unclaimed tokens after distribution periods
- Close accounts and reclaim rent

This revised model provides extremely strong on-chain guarantees about the immutability and verifiability of each campaign instance.

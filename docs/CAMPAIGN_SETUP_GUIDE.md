# Prism Protocol: Campaign Setup Guide for Operators (Highly Automated)

This guide walks you through the highly automated process of setting up and launching a token distribution campaign using the Prism Protocol. The `prism-cli` handles most of the heavy lifting.

**Core Principle: On-Chain Immutability via Merkle Identifiers**

Prism Protocol enforces strong immutability for its on-chain records. A Campaign and its Cohorts are uniquely identified by cryptographic hashes, ensuring that a specific on-chain Campaign PDA represents a fixed and unalterable set of distribution rules.

*   **Cohort Immutability:** Each `Cohort` PDA is uniquely identified by its parent Campaign and its specific Merkle distribution root (`merkle_root`). This root is generated from a list of claimants and their entitlements for that cohort. Once a `Cohort` is initialized on-chain with its `merkle_root`, its distribution logic is immutable.

*   **Campaign Immutability (`campaign_fingerprint`):** A `Campaign` PDA is uniquely identified on-chain by a `campaign_fingerprint`. This identifier is a cryptographic hash derived from the Merkle roots of *all* its constituent Cohorts. This means a `Campaign` PDA inherently represents a specific, immutable collection of Cohort distributions. 
    *   **Generation:** The `prism-cli` will first calculate the `merkle_root` for every cohort you define. Then, it will deterministically combine these cohort Merkle roots (e.g., by sorting them, concatenating, and hashing) to produce the single `campaign_fingerprint`.
    *   This `campaign_fingerprint` is then used as a seed to create the unique `Campaign` PDA.

*   **Handling Changes/Updates/New Waves:** 
    *   If you need to modify a specific cohort's distribution (e.g., add new claimants, change rewards), you will generate a *new* `merkle_root` for that cohort, leading to a *new, distinct* `Cohort` PDA for that updated distribution.
    *   If you want to add a new cohort to what you logically consider an ongoing campaign, or if any existing cohort's Merkle root changes, the overall set of cohort Merkle roots changes. This will result in a *new* `campaign_fingerprint` being calculated by the CLI. Consequently, you will initialize a *new, distinct* `Campaign` PDA. The original `Campaign` PDA and its associated Cohorts remain untouched and immutable.
    *   Think of each `Campaign` PDA as a complete, versioned snapshot of a set of distributions.

This model ensures maximum transparency and verifiability. The primary on-chain control after launch is the `is_active` flag on a `Campaign` PDA (acting as an on/off switch for all its cohorts) and the eventual ability to withdraw unclaimed funds.

## Phase 1: Campaign & Cohort Definition (Off-Chain)

Your main focus is defining the campaign's overall parameters, the token to be distributed, and the recipient lists for each distinct cohort.

1.  **Define Overall Campaign Parameters (Conceptual):**
    *   **Campaign Name (for CLI & organization, e.g., `campaign_name_metadata`):** A human-readable name for your overall distribution effort (e.g., "PROJECT_ALPHA_Q1_REWARDS"). This is for your reference and for naming CLI output files. It is **not** stored directly on the `Campaign` PDA as its primary identifier.
    *   **Token Mint (`mint`):** The Pubkey of the single SPL token you will be distributing. This is a **required** top-level parameter for the `prism-cli` and will be stored on the `Campaign` PDA.
    *   **Authority Keypair:** The keypair that will pay for transactions and be set as the `authority` on the `Campaign` PDA (allowing actions like toggling `is_active`).
    *   **(Optional) Claim Deadline (`claim_deadline_timestamp`):** A global deadline.
    *   **(Optional) Max Claimants Per Vault (`max_claimants_per_vault`):** A hint for vault calculation.

2.  **Define Individual Cohorts & Compile Claimant Lists:**
    *   For each distinct distribution group (cohort) you want within this campaign instance:
        *   **Cohort Name (for CLI & organization, e.g., `cohort_name_metadata`):** A human-readable name for this specific cohort (e.g., "EarlyContributors", "NFT Holders Tier1"). This is for your reference and for CLI output. It is **not** stored on the `Cohort` PDA.
        *   **Reward Amount (`reward_per_entitlement`):** The fixed amount of the `mint` tokens per entitlement.
        *   **Claimant List File:** As previously described (CSV with pubkeys, or pubkeys + entitlement counts).

3.  **Prepare the Campaign Configuration File (`campaign_config.yaml`):**
    *   This file instructs the `prism-cli`.
    *   **Example `campaign_config.yaml` structure:**
      ```yaml
      campaign_name_metadata: "PROJECT_ALPHA_Q1_REWARDS" # For your reference & CLI output
      mint: "YourSplTokenMintPubkey..." # Required
      authority_keypair_path: "/path/to/your_authority_keypair.json" # Path to the authority keypair
      # claim_deadline_timestamp: "2025-03-31T23:59:59Z" # Optional
      # max_claimants_per_vault: 10000 # Optional global override

      cohort_definitions:
        - cohort_name_metadata: "CORE_CONTRIBUTORS"
          reward_per_entitlement: 1000000000 
          claimants_file: "./claimants/core_contributors.csv"
          # number_of_vaults_to_use: 5 # Optional

        - cohort_name_metadata: "COMMUNITY_MVPS"
          reward_per_entitlement: 250000000
          claimants_file: "./claimants/community_mvps.csv"
      ```

## Phase 2: Execute `prism-cli` – Generate Merkle Trees, Campaign Identifier, & Action Plan

The `prism-cli` performs several critical steps:

1.  **Run `prism-cli`:**
    *   `prism-cli process-campaign campaign_config.yaml`

2.  **CLI Actions (Fully Automated):**
    *   Parses `campaign_config.yaml`.
    *   **For each cohort defined in `cohort_definitions`:**
        *   Processes the claimant list file, determines `count_of_entitlements`.
        *   Calculates necessary vaults and assigns claimants consistently.
        *   Generates the Merkle tree, yielding a unique `merkle_root` for this cohort.
        *   Calculates token funding needs for this cohort.
        *   Generates Merkle proofs for each claimant in this cohort.
    *   **Calculate `campaign_fingerprint`:**
        *   Collects all individual cohort `merkle_root`s generated above.
        *   Sorts these roots (e.g., lexicographically).
        *   Concatenates the sorted roots.
        *   Hashes the concatenated string (e.g., SHA256) to produce the single, unique `campaign_fingerprint` ([u8; 32]). This identifier represents this entire, specific set of cohort distributions.

3.  **Review `prism-cli` Output (The "Action Plan & Report"):**
    *   The CLI provides:
        *   **`campaign_fingerprint`**: The calculated [u8; 32] hash. This is crucial for on-chain initialization.
        *   **On-Chain Initialization Parameters:**
            *   For `initialize_campaign`: The `campaign_fingerprint`, `mint`.
            *   For each `initialize_cohort`: The `campaign_fingerprint` (to find parent Campaign PDA), its specific cohort `merkle_root`, `reward_per_entitlement`, `vault_pubkeys`.
        *   **Vault Operations Report, SPL Token Commands, Lookup Files:** (Similar to before, but lookup files might be organized under a directory named after `campaign_name_metadata` or the `campaign_fingerprint` stringified).

## Phase 3: Fund Vaults, Delegate Authority, & Initialize On-Chain

1.  **Fund Vaults & Delegate Authority:** (As before, guided by CLI output).

2.  **Initialize Campaign & Cohorts On-Chain:**
    *   **Understanding On-Chain Identifiers:**
        *   The `Campaign` PDA will be uniquely identified on-chain by the `campaign_fingerprint` (and program ID). This is the `seeds = [b"campaign", campaign_fingerprint.as_ref()]`.
        *   Each `Cohort` PDA will be uniquely identified by its parent `Campaign` PDA's key and its own specific `merkle_root`. (`seeds = [b"cohort", campaign_pda.key().as_ref(), cohort_merkle_root.as_ref()]`).
    *   **Execution (Manual or CLI-assisted):**
        *   Call `initialize_campaign` with the `campaign_fingerprint` and `mint` (signed by authority).
        *   For each cohort: Call `initialize_cohort` with the `campaign_fingerprint`, its cohort `merkle_root`, `reward_per_entitlement`, and `vaults` (signed by authority).

## Phase 4: Distribute Lookup Information & Go Live

1.  **Host Lookup Files:** (As before).
2.  **Integrate Lookup Mechanism into dApp:**
    *   The dApp needs the `campaign_fingerprint` of the target campaign instance.
    *   For a user, the dApp queries lookup data. For each eligible cohort, it retrieves: cohort `merkle_root`, Merkle proof, `assigned_vault_pubkey`, `count_of_entitlements`.
    *   The dApp constructs `claim_reward` transaction with: `campaign_fingerprint`, cohort `merkle_root`, proof, vault, entitlements.

## Phase 5: Post-Campaign Management (Optional)

(As before: monitor, withdraw, close accounts).

This revised model provides extremely strong on-chain guarantees about the immutability and verifiability of each campaign instance.

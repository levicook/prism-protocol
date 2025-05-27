# Prism Protocol: Design Notes & Decisions

> **⚠️ DOCUMENTATION STATUS:** This document describes an earlier design iteration and is currently **OUTDATED**. It will be updated in a future iteration to match the current implementation which uses:
> - `CampaignV0` / `CohortV0` / `ClaimReceiptV0` account structures
> - `campaign_fingerprint: [u8; 32]` instead of `campaign_id: String`
> - Different instruction naming and account field structures
> - The actual implementation can be found in `programs/prism-protocol/src/`

## 1. Core Goal

To enable Solana projects to efficiently distribute tokens to large audiences by minimizing write contention and optimizing on-chain resources for SPL token airdrops and rewards.

## 2. System Overview

- **On-Chain Program (`Prism Protocol`):** An Anchor-based Solana program that manages `Cohort` configurations, verifies claimant eligibility via Merkle proofs, and facilitates SPL token transfers from pre-funded, shared token vaults.
- **Off-Chain CLI (`prism-cli`):** A Rust-based command-line tool for project administrators to:
  - Define logical reward cohorts (reward amount, token mint).
  - Take a list of recipients for a cohort.
  - Generate a global Merkle tree for all recipients in that cohort.
  - Perform consistent hashing to map each recipient to one of several pre-specified SPL token vault pubkeys for that cohort.
  - Calculate the exact amount of tokens each vault needs to be funded with.
  - Output a comprehensive "initialization plan" including data for creating the on-chain `Cohort` account, per-vault funding amounts, and a lookup file (proofs and assigned vaults for each claimant).
- **Claimant UI/Frontend Integration:** Projects will integrate with Prism Protocol, typically by:
  - Hosting the CLI-generated lookup file (mapping users to proofs and their assigned vaults).
  - Providing a UI for users to connect their wallets.
  - A lookup mechanism (e.g., API or client-side logic using the hosted file) to fetch a user's specific proof, their assigned vault pubkey for the claim, and the relevant `Cohort` account.
  - Constructing and sending the `claim_reward` transaction.

## 3. Key Design Decisions & Rationale

### 3.1. Minimizing Write Contention & Scaling Distributions: The `Cohort` Model

- **Problem:** Standard airdrops to large audiences often suffer from write contention, primarily on shared SPL Token Accounts (Vaults) and global state accounts.
- **Key Insight:** `Claim` PDAs ("claim receipts"), when seeded uniquely per claimant and per `Cohort` (e.g., `["claim", cohort_pubkey, claimant_pubkey]`), are inherently uncontended as each claimant interacts with their own unique receipt account. The primary remaining contention point is the token vault(s).

- **Chosen Architecture: Single `Cohort` with Client-Side Consistent Hashing & Pre-Calculated Vault Funding**

  - **Concept:** A single on-chain account, `Cohort`, defines an entire logical reward cohort. It holds a global Merkle root for all recipients in that cohort and a list of multiple SPL token vault pubkeys. The `prism-cli` pre-calculates how many claimants map to each vault via consistent hashing. Each of these pre-specified vaults is then funded by the project authority with the _exact_ amount of tokens it needs for its assigned claimants.
  - **State:**
    - `Cohort` (1 PDA per logical cohort):
      - `authority: Pubkey` (can manage this cohort).
      - `cohort_id: String` (e.g., "OG_SUPPORTERS_2024_Q3").
      - `merkle_root: [u8; 32]` (for _all_ recipients in the cohort).
      - `vault_pubkeys: Vec<Pubkey>` (e.g., 5-N SPL token accounts designated for this cohort).
      - `reward_token_mint: Pubkey`.
      - `reward_amount_per_claimant: u64`.
      - `total_claimants: u64` (total number of unique recipients in the Merkle tree for this cohort).
      - `creation_timestamp: i64`.
      - `claim_deadline_timestamp: Option<i64>`.
      - `is_active: bool`.
      - `bump: u8`.
    - `Claim` (PDA: `["claim", cohort_pubkey, claimant_pubkey]`) - unique per claimant for this cohort, created on first claim.
    - _Individual SPL Token Vault Accounts:_ Each pubkey in `Cohort.vault_pubkeys` corresponds to an SPL token account. These accounts are funded by the project authority _prior to or during cohort activation_. They are not PDAs owned by the `Cohort` account itself but are managed externally by the authority.
  - **Setup Process (CLI intensive):**
    1.  **Define Cohort & Vaults:** The project authority decides on the cohort parameters (ID, reward, token, deadline) and designates a list of existing SPL token accounts to serve as vaults for this cohort.
    2.  **CLI Processing:**
        - Takes the full list of `claimant_pubkeys` for the cohort and the list of designated `vault_pubkeys`.
        - Generates the single, global Merkle tree for all claimants.
        - For each `claimant_pubkey`, it performs consistent hashing against the `vault_pubkeys` list to deterministically map the claimant to one vault.
        - It tallies the exact number of claimants (`N_v`) assigned to each Vault `v`.
        - It calculates the precise funding needed for each Vault `v`: `N_v * reward_amount_per_claimant`.
    3.  **CLI Output ("Initialization Plan"):**
        - Parameters for the `initialize_cohort` instruction (authority, cohort_id, merkle_root, vault_pubkeys list, reward_token_mint, reward_amount_per_claimant, total_claimants, deadline).
        - For each designated vault pubkey: the exact amount of tokens it requires.
        - A lookup file (e.g., JSON) mapping each `claimant_pubkey` to their Merkle proof and their assigned `vault_pubkey`.
    4.  **On-Chain Initialization & Funding:**
        - The project authority uses the CLI output to call the `initialize_cohort` instruction, creating the `Cohort` account.
        - The authority ensures each designated vault (from `vault_pubkeys`) is funded with the precise token amount calculated by the CLI. This funding happens externally to the `initialize_cohort` instruction.
  - **Claim Process:**
    1.  **Off-Chain (Client/Frontend/Lookup Service):**
        - Using the lookup file/service, the dApp retrieves the claimant's Merkle proof and their pre-assigned `vault_pubkey` for the specific cohort.
    2.  **On-Chain (`claim_reward` Instruction):**
        - Transaction includes the `Cohort` account, the _one specific pre-assigned vault pubkey_ (retrieved off-chain), the claimant's token account, and the Merkle proof.
        - The program verifies the Merkle proof against `Cohort.merkle_root`.
        - It verifies that the provided `assigned_vault_pubkey` is part of the `Cohort.vault_pubkeys` list.
        - It creates/checks the `Claim` PDA for the claimant and cohort.
        - It transfers tokens from the `assigned_vault_pubkey` (which acts as the source vault for this specific claim) to the claimant.
  - **Advantages:**
    - **Single Merkle Root per Cohort:** Simplifies Merkle tree generation and management.
    - **Effective Write Load Distribution on Vaults:** Write load is distributed as claimants target their specific, pre-assigned vault. Contention on any single vault is minimized.
    - **Precise Funding & Clear Accountability:** Each vault's required funding is known upfront. The project authority is responsible for this precise external funding. If an assigned vault is empty, it means either all its assigned claimants have claimed or it was underfunded by the authority.
    - **No Client-Side Fallback Logic Needed for Empty Vaults:** The assigned vault _is_ the correct vault. The transaction will fail if that specific vault cannot service the claim (e.g. insufficient funds).
    - **Simpler On-Chain Cohort Configuration:** Only one `Cohort` account per cohort.
  - **Considerations & Challenges:**
    - **CLI & Lookup Service Criticality:** The consistent hashing logic (in CLI and potentially replicated in a lookup service or client-side) and the integrity of the lookup file are crucial.
    - **Immutability of Vault List Post-Launch:** Once a cohort is active and vaults are funded based on the consistent hash mapping, the `vault_pubkeys` list in `Cohort` should generally be considered immutable for that active distribution. Changing it would remap users and invalidate the precise funding plan. A new cohort would be required for changes.
    - **External Vault Funding Responsibility:** The protocol relies on the project authority to correctly fund the designated vaults. The on-chain program does not manage the funding of these external vaults during `initialize_cohort`.
    - **Slightly Uneven Distribution by Consistent Hashing:** Consistent hashing aims for good distribution but might not be perfectly equal. The CLI calculates the _actual_ number of claimants and thus the exact funding for each vault, so this is an accounting detail managed by the CLI and project authority.

- **Previous Model B (Sharded `DistributionShard` Accounts) is archived as Model A provides a more streamlined on-chain approach while achieving similar contention mitigation through off-chain coordination.**

### 3.2. Merkle Tree & Proof System

- **Purpose:** To efficiently verify on-chain that a claimant is part of an authorized list without storing the entire list on-chain.
- **Hashing Algorithm:** SHA256, following SPL Merkle tree conventions:
  - **Leaves:** `SHA256(0x00_PREFIX || recipient_pubkey.to_bytes())`
  - **Intermediate Nodes:** `SHA256(0x01_PREFIX || sorted_child_hash_1 || sorted_child_hash_2)` (children hashes are sorted lexicographically before concatenation).
- **Leaf Content:** The leaf itself is the `recipient_pubkey` (derived from the `signer` of the claim transaction). The `reward_amount_per_claimant` is fixed per `DistributionCohort`.
- **Proof Format:** `Vec<[u8; 32]>` (a flat list of SHA256 hashes representing the sibling nodes up to the root).
- **CLI Merkle Library:** `rs-merkle` (Rust crate) combined with a custom `Hasher` implementation (`SplSha256Algorithm` in `prism-cli`) that replicates the SPL SHA256 prefixing and node combination logic. This allows for easy extraction of `Vec<[u8; 32]>` proofs.

### 3.3. On-Chain State

#### A. `Campaign` Account (PDA) - NEW

- Represents an overall distribution campaign, which can group multiple `DistributionCohort`s.
- Manages the lifecycle (active status, deadline) and authority for all cohorts under it.
- **Seeds (Example):** `["campaign", authority: Pubkey, campaign_id: String]`
  - `authority`: The wallet that can create and manage this campaign and its cohorts.
  - `campaign_id`: A unique string identifier for this campaign (e.g., "PROJECT_GENESIS_DROP_2024", max 64 bytes).
- **Fields:**
  - `authority: Pubkey`
    - The admin who initialized and can manage this `Campaign`.
  - `campaign_id: String` (Max length TBD, e.g., 64 bytes)
    - As in seeds. Stored for easier off-chain querying.
  - `creation_timestamp: i64`
    - Unix timestamp of when this campaign was initialized.
  - `claim_deadline_timestamp: Option<i64>`
    - Optional deadline after which claims are no longer accepted for *any cohort* in this campaign.
  - `is_active: bool`
    - Allows pausing/unpausing *all cohorts* within this campaign.
  - `bump: u8` (for the `Campaign` PDA itself)

#### B. `DistributionCohort` Account (PDA) (Formerly `DistributionTier`)

- Represents a single, logical reward cohort *within a Campaign*.
- **Critical Immutability Note:** Once a `DistributionCohort` is initialized and especially after the first claim, fields like `merkle_root` and `vault_pubkeys` **must be considered immutable**. Changing them would break consistent hashing, invalidate proofs, and/or lead to incorrect funding access for claimants. For MVP, no updates to these fields will be allowed post-initialization.
- **Seeds (Example):** `["distribution_cohort", campaign_account: Pubkey, cohort_id_str: String]`
  - `campaign_account`: The `Pubkey` of the parent `Campaign` account.
  - `cohort_id_str`: A unique string identifier for this cohort *within the campaign* (e.g., "GOLD_COHORT", "OG_MEMBERS", max 64 bytes).
- **Fields:**
  - `campaign: Pubkey`
    - Pubkey of the parent `Campaign` account.
  - `cohort_id_str: String`
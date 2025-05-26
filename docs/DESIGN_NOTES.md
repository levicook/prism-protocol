# Prism Protocol: Design Notes & Decisions

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
  - `cohort_id_str: String` (Max length TBD, e.g., 64 bytes)
    - As in seeds.
  - `merkle_root: [u8; 32]`
    - The single Merkle root for *all* claimants in this specific cohort. (Immutable post-initialization)
  - `vault_pubkeys: Vec<Pubkey>` (Max N items, e.g., N=10)
    - A list of pre-specified, externally owned SPL token account pubkeys that will serve as the shared vaults for this cohort. These are funded by the project authority. (Immutable post-initialization)
  - `reward_token_mint: Pubkey`
    - Mint of the token being distributed for this cohort.
  - `reward_amount_per_claimant: u64`
    - The fixed amount of tokens each claimant in this cohort receives.
  - `total_claimants: u64`
    - The total number of unique recipients (leaves in the Merkle tree) for this cohort. Set at initialization.
  - `cohort_creation_timestamp: i64`
     - Unix timestamp of when this specific cohort was initialized.
  - `bump: u8` (for the `DistributionCohort` PDA itself)

#### C. `ClaimReceipt` Account (PDA) (Formerly `ClaimStatus`)

- Tracks that a specific claimant has successfully claimed from a specific `DistributionCohort`.
- **Its existence signifies a completed claim.**
- **Seeds (Example):** `["claim_receipt", distribution_cohort_pubkey: Pubkey, claimant_pubkey: Pubkey]`
  - `distribution_cohort_pubkey`: The address of the `DistributionCohort` account this claim receipt belongs to.
  - `claimant_pubkey`: The recipient's address (the signer of the `claim_reward` transaction).
- **Fields:**
  - `campaign: Pubkey` (Copied from `DistributionCohort` for easier querying)
  - `distribution_cohort: Pubkey` (The cohort this receipt is for)
  - `claimant: Pubkey` (The one who claimed)
  - `amount_claimed: u64` (Stores `reward_amount_per_claimant` from the cohort at the time of claim)
  - `claim_timestamp: i64`
  - `claimed_from_vault: Pubkey` (The specific vault pubkey from `DistributionCohort.vault_pubkeys` that this claim was processed against)
  - `bump: u8`

### 3.4. On-Chain Instructions

**Campaign Management Instructions:**

1.  **`initialize_campaign`** (NEW)
    - **Context:** `authority` (signer, fee payer), `system_program`, `rent_sysvar`.
    - **Args:**
      - `campaign_id: String`
      - `claim_deadline_timestamp: Option<i64>`
      - `initial_is_active: bool` (Typically `true`)
    - **Action:**
      - Creates and initializes the `Campaign` PDA.
      - Sets `authority` to the signer.
      - Stores `campaign_id`, `claim_deadline_timestamp`, `is_active = initial_is_active`.
      - Sets `creation_timestamp`.

2.  **`update_campaign_settings`** (NEW)
    - **Context:** `authority` (signer, must match `Campaign.authority`), `campaign`.
    - **Args:**
      - `new_claim_deadline_timestamp: Option<Option<i64>>` (Outer `Option` to signify if updating, inner `Option` for the value itself)
      - `new_is_active: Option<bool>`
    - **Action:** Allows `Campaign.authority` to update `claim_deadline_timestamp` and `is_active` status for the campaign.

**Cohort & Claiming Instructions:**

3.  **`initialize_cohort`** (Modified, formerly `initialize_tier`)
    - **Context:**
      - `campaign_authority` (signer, fee payer, must match `Campaign.authority`)
      - `campaign` (The `Campaign` PDA this cohort belongs to, read-only, used for validation and to copy `Campaign.authority` for checks)
      - `system_program`
      - `rent_sysvar`
    - **Args:**
      - `cohort_id_str: String`
      - `merkle_root: [u8; 32]`
      - `vault_pubkeys: Vec<Pubkey>` (List of *externally owned and already funded* SPL token accounts. Authority must ensure delegation is set up.)
      - `reward_token_mint: Pubkey`
      - `reward_amount_per_claimant: u64`
      - `total_claimants: u64`
    - **Action:**
      - Verifies `campaign_authority` matches `campaign.authority`.
      - Creates and initializes the `DistributionCohort` PDA, associating it with the `campaign`.
      - Stores all provided arguments. Sets `cohort_creation_timestamp`.
      - **Important Immutability:** This instruction sets the core, immutable parameters of the cohort. The `campaign_authority` is responsible for ensuring vaults are funded and transfer authority delegated *before* claims begin.

4.  **`claim_reward`** (Modified)
    - **Context:**
      - `claimant` (signer, fee payer for `ClaimReceipt` creation)
      - `campaign` (The `Campaign` PDA, read-only for checks)
      - `distribution_cohort` (The `DistributionCohort` PDA for this cohort)
      - `claim_receipt` (PDA for this claimant & cohort, to be created)
      - `assigned_token_vault` (The specific vault from `distribution_cohort.vault_pubkeys` that this claimant is mapped to. *Must be writable*.)
      - `claimant_destination_token_account` (Claimant's ATA for `reward_token_mint`, *must be writable*).
      - `token_program`
      - `system_program`
      - `rent_sysvar`
    - **Args:**
      - `proof: Vec<[u8; 32]>`
      - `assigned_vault_index: u8` (Index of `assigned_token_vault` in `distribution_cohort.vault_pubkeys`)
    - **Action:**
      - Basic checks: `campaign.is_active`, `campaign.claim_deadline_timestamp` (if set).
      - Verify that `distribution_cohort.campaign` matches `campaign.key()`.
      - Verify that `distribution_cohort.vault_pubkeys[assigned_vault_index]` equals `assigned_token_vault.key()`.
      - Calculates `leaf_hash` from `claimant.key()` using SPL SHA256 style.
      - Verifies `proof` against `distribution_cohort.merkle_root` and `leaf_hash`.
      - Initializes `ClaimReceipt` PDA (payer = claimant). Fails if it already exists.
      - CPI to Token Program: Transfer `distribution_cohort.reward_amount_per_claimant` from `assigned_token_vault` to `claimant_destination_token_account`.
        - This requires that `assigned_token_vault` has delegated transfer authority (for at least the claimable amount) to the `DistributionCohort` PDA itself, or a PDA derived from `DistributionCohort` (e.g., `["vault_delegate", distribution_cohort.key()]`) which would then be the signer for the CPI.
      - Populates `ClaimReceipt` fields: `campaign`, `distribution_cohort`, `claimant`, `amount_claimed`, `claim_timestamp`, `claimed_from_vault`.

5.  **`update_cohort_settings`** --- **REMOVED FOR MVP.** Updating core cohort settings post-initialization is too risky. To change `merkle_root`, `vault_pubkeys`, etc., a new cohort must be created.

6.  **`withdraw_unclaimed_funds_from_cohort_vault`** (Modified for clarity, formerly `withdraw_unclaimed_funds_from_tier_vault`)
    - **Context:**
      - `caller_authority` (signer - This must be the authority of the specific `token_vault_to_withdraw_from` OR the `Campaign.authority` if the vault's authority was delegated to the campaign/cohort system for this purpose).
      - `campaign` (Read-only, for deadline/status checks if needed, and to verify `Campaign.authority` if applicable).
      - `distribution_cohort` (Read-only, to confirm vault is part of this cohort).
      - `token_vault_to_withdraw_from` (writable, must be one of `distribution_cohort.vault_pubkeys`).
      - `destination_token_account` (Writable, belonging to `caller_authority`).
      - `token_program`.
    - **Action:**
      - Typically called after `campaign.claim_deadline_timestamp` has passed or `campaign.is_active` is false.
      - Verifies `token_vault_to_withdraw_from` is part of `distribution_cohort.vault_pubkeys`.
      - Transfers remaining balance from `token_vault_to_withdraw_from` to `destination_token_account`.
      - This instruction facilitates withdrawal from one vault at a time. The responsible authority calls it for each vault.
      - **Note:** This does not close any PDAs (`Campaign`, `DistributionCohort`, `ClaimReceipt`). Vaults are external.

## 4. `prism-cli` Functionality

- Input:
  - `campaign_id: String` (For the overall campaign these cohorts belong to).
  - For each cohort to be created/managed:
    - `cohort_id_str: String` (Identifier for this specific cohort within the campaign).
    - List of recipient pubkeys (e.g., from a CSV).
    - `reward_token_mint: Pubkey`.
    - `reward_amount_per_claimant: u64`.
    - A list of `vault_pubkeys: Vec<Pubkey>` (pre-existing SPL token accounts that the project authority will fund and whose transfer authority will be delegated, see below).
- General Campaign Settings (applied if initializing a new campaign):
  - `claim_deadline_timestamp: Option<i64>`.
  - `initial_is_active: bool`.

- Processing:
  1.  **Campaign Setup (if new):**
      - If the `campaign_id` is new, prepare parameters for `initialize_campaign`.
  2.  **For each Cohort:**
      - Validate inputs (e.g., ensure `vault_pubkeys` list is not empty for the cohort).
      - Generate a single Merkle tree for all `recipient_pubkeys` for this cohort using `SplSha256Algorithm`.
      - Extract the global Merkle root for this cohort.
      - For each `recipient_pubkey` in this cohort:
          - Perform consistent hashing against the cohort's `vault_pubkeys` list to determine which vault that claimant is deterministically mapped to. Store the index of this vault.
          - Generate their individual `Vec<[u8; 32]>` proof for this cohort.
      - Tally the exact number of claimants assigned to each vault within this cohort and calculate the precise funding required for each vault (`assigned_claimants_for_vault * reward_amount_per_claimant`).
- Output ("Campaign & Cohort Initialization Plan"):
  - **For On-Chain `initialize_campaign` instruction (if new campaign):**
    - `campaign_id`
    - `claim_deadline_timestamp`
    - `initial_is_active`
  - **For each Cohort (for On-Chain `initialize_cohort` instruction):**
    - `parent_campaign_pubkey` (once known or if pre-calculable)
    - `cohort_id_str`
    - `merkle_root`
    - The `vault_pubkeys` list for this cohort
    - `reward_token_mint`
    - `reward_amount_per_claimant`
    - `total_claimants` (count of unique recipient pubkeys for this cohort)
  - **For Project Authority Action (Funding & Delegation - Per Cohort, Per Vault):**
    - For each `vault_pubkey` in each cohort's list:
      - The exact amount of `reward_token_mint` tokens it must be funded with.
      - A reminder/instruction to delegate transfer authority of this vault (for at least the calculated amount) to the `DistributionCohort` PDA (or its derived authority PDA, once its address is known/pre-calculable for that cohort). This delegation is crucial for the `claim_reward` CPI.
  - **Lookup Files (e.g., `lookup_campaign_id_cohort_id_str.json` - one per cohort):**
    - An array or map where each entry contains:
      - `recipient_pubkey: String`
      - `proof: Vec<String>` (hashes as strings)
      - `assigned_vault_pubkey: String` (the pubkey of the vault they are mapped to for this cohort)
      - `assigned_vault_index: u8` (the index of their vault in the `DistributionCohort.vault_pubkeys` array for this cohort)
      - `reward_amount: u64`
      - `token_mint: String`
      - `distribution_cohort_pubkey: String` (once known)
      - `campaign_pubkey: String` (once known)

## 5. Open Questions / Areas for Further Refinement

- **Clarity on Immutability:** The notes on immutability for `DistributionCohort` fields (`merkle_root`, `vault_pubkeys`) are now in place. This needs to be strictly enforced in the on-chain program.
- **Vault Transfer Authority Delegation for `claim_reward`:** This remains a critical point. The CLI must provide clear instructions to the admin on how to set up `spl-token approve` (or equivalent) for each vault, delegating to the correct `DistributionCohort` PDA (or its derived vault delegate PDA). The address of this delegate PDA must be predictable or determined before the admin can set up the delegation. A derived PDA like `["vault_delegate", distribution_cohort_pda.key()]` is a good candidate for the CPI signer.
- **`Campaign` Authority vs. `DistributionCohort` Logic:** Ensure the roles are clear. `Campaign` authority manages the campaign lifecycle. `DistributionCohort` logic primarily handles Merkle proof verification and state for a specific set of claimants and rewards.
- **Account Closing & Rent Reclamation:**
    - `Campaign` PDA can be closed by its authority if `is_active = false`, deadline passed, and all associated cohorts are considered complete/dealt with.
    - `DistributionCohort` PDAs could potentially be closed by `Campaign.authority` under similar conditions (though no funds are held directly by `DistributionCohort`).
    - `ClaimReceipt` accounts are paid for by claimants. No central reclamation.
    - Vault accounts are external; their lifecycle is managed by their owners.
- **Gas/Compute Optimization:** Still relevant, especially for `claim_reward`.
- **Error Handling:** Comprehensive on-chain error codes.
- **Frontend Lookup Mechanism:** Standardize the structure of the lookup files.
- **Idempotency for `initialize_campaign` and `initialize_cohort`**: Ensure safe retries.
- **Security Audits:** Crucial.
- **Maximum number of vaults in `DistributionCohort.vault_pubkeys`:** Reconfirm limit.
- **String Lengths for IDs:** Define and enforce maximum lengths for `campaign_id` and `cohort_id_str` to manage account size.

## 6. Out of Scope (Initial Version - Prism MVP)

- Dynamic reward amounts per user within the same `DistributionCohort`.
- Complex vesting schedules.
- On-chain governance of the Prism protocol itself.
- Direct distribution of NFTs.
- The Prism program creating/owning the SPL token vaults.
- Automatic rebalancing of funds between vaults.
- **Updates to core `DistributionCohort` settings (`merkle_root`, `vault_pubkeys`, etc.) after initialization.** Changes require a new cohort.

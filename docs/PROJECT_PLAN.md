# Prism Protocol: Project Plan & Checklist

## 1. Core Project Goal

To enable efficient, scalable, and verifiable token distribution on Solana, minimizing write contention and optimizing on-chain resources. (See `README.md` for full details).

## 2. Core Components - Implementation Checklist

### On-Chain Program (`programs/prism-protocol/src/`)

*   **State Accounts (`state.rs`):**
    *   [x] `Campaign` struct defined
    *   [x] `CohortV0` struct defined (Baseline Cohort)
    *   [ ] `CohortV1` struct defined (e.g., with "Primary Trunks" for optimization) - *Design & To Do*
    *   [x] `ClaimReceipt` struct defined
*   **Instructions (`instructions/` & `lib.rs`):**
    *   [ ] `handle_initialize_campaign` - *To Do*
    *   [ ] `handle_initialize_cohort_v0` - *To Do*
    *   [ ] `handle_initialize_cohort_v1` - *Design & To Do*
    *   [x] `handle_claim_tokens_v0` - Implemented (based on current `claim_tokens.rs`)
    *   [ ] `handle_claim_tokens_v1` - *Design & To Do*
    *   [ ] `handle_set_campaign_active_status` - *To Do*
    *   [ ] `handle_reclaim_tokens` - *Design & To Do* (See section 3)
*   **Merkle Logic:**
    *   [x] `ClaimLeaf` struct and `hash_claim_leaf` function (`merkle_leaf.rs`) - Implemented & Tested
    *   [x] `verify_merkle_proof` function (in `claim_tokens.rs`, for V0) - Implemented
    *   [ ] Modified Merkle verification for `claim_tokens_v1` (if using trunks) - *Design & To Do*
*   **Program Entrypoint (`lib.rs`):**
    *   [x] Declare program ID
    *   [x] Define `claim_tokens_v0` public instruction (renaming current `claim_tokens`)
    *   [ ] Define `claim_tokens_v1` public instruction - *Design & To Do*
    *   [ ] Define `initialize_campaign` public instruction - *To Do*
    *   [ ] Define `initialize_cohort_v0` public instruction - *To Do*
    *   [ ] Define `initialize_cohort_v1` public instruction - *Design & To Do*
    *   [ ] Define `set_campaign_active_status` public instruction - *To Do*
    *   [ ] Define `reclaim_tokens` public instruction - *To Do*

### Off-Chain CLI (`prism-cli`)

*   **Status:** *To Be Designed & Implemented*
*   **Core Functionality Checklist:**
    *   [ ] Campaign configuration file parsing (`campaign_config.yaml` or similar)
    *   [ ] Claimant list processing (CSV, JSON, etc.)
    *   [ ] `ClaimLeaf` data generation (claimant, assigned_vault, entitlements)
    *   [ ] Merkle tree generation for each cohort -> `merkle_root` (for `CohortV0`)
    *   [ ] (If applicable) Merkle tree and trunk hash generation for `CohortV1`
    *   [ ] `campaign_fingerprint` calculation (from sorted cohort `merkle_root`s)
    *   [ ] Individual Merkle proof generation for each claimant (for `V0` and `V1` paths)
    *   [ ] Output: Parameters for on-chain `initialize_campaign` & `initialize_cohort_v0`/`v1`
    *   [ ] Output: Vault funding requirements report
    *   [ ] Output: Claimant lookup files (proofs, assigned_vaults, entitlements, etc.)

### Testing (using Mollusk)

*   **Unit Tests (On-Chain):**
    *   [x] `merkle_leaf.rs` tests for `hash_claim_leaf`
    *   [ ] Tests for PDA derivation logic
    *   [ ] Tests for other critical utility functions
*   **Integration Tests (Anchor `tests/` using Mollusk):**
    *   [ ] Full `initialize_campaign` instruction test
    *   [ ] Full `initialize_cohort_v0` instruction test
    *   [ ] Full `initialize_cohort_v1` instruction test (once designed)
    *   [ ] Full `claim_tokens_v0` instruction test (various scenarios)
    *   [ ] Full `claim_tokens_v1` instruction test (once designed, various scenarios)
    *   [ ] Full `set_campaign_active_status` instruction test
    *   [ ] Full `reclaim_tokens` instruction test (once design is finalized)
    *   [ ] End-to-end test (V0 path): CLI generates data -> on-chain setup -> successful claim -> reclaim.
    *   [ ] End-to-end test (V1 path, once designed): CLI generates data -> on-chain setup -> successful claim -> reclaim.

## 3. Key Design Decisions & Open Questions

*   **[ ] `reclaim_tokens` Strategy:**
    *   **Decision Needed:** Granular vault-by-vault withdrawal (see `DESIGN_NOTES.md`, section 3.4, item 6) vs. a single `reclaim_tokens` instruction targeting a full cohort or campaign (as per `lib.rs` `ReclaimTokens` struct arguments).
    *   **Considerations:** Authority model, flexibility, atomicity.
*   **[ ] `CohortV1` Design ("Primary Trunks") Details:**
    *   **Action:** Design how "primary trunks" are stored in `CohortV1` and how `verify_merkle_proof` is adapted for `claim_tokens_v1`.
    *   **Trigger:** Proceed after initial benchmarking of `V0` shows potential for significant gains.

## 4. Benchmarking Plan (using Mollusk)

*   **Objective:** Quantitatively validate performance, scalability, and resource consumption of `V0` and `V1` protocol versions. Inform optimization decisions.
*   **Methodology:** Develop standardized test scripts using Mollusk for on-chain benchmarks and separate scripts for `prism-cli` performance if needed.
*   **On-Chain Benchmarking Areas:**
    *   **`claim_tokens_v0` vs `claim_tokens_v1` Instruction:**
        *   [ ] **CU Consumption vs. Proof Length/Trunk Usage:**
            *   Measure CUs for cohorts of varying sizes (e.g., 100, 1k, 10k, 100k, 1M, 5M claimants) for both V0 and V1.
        *   [ ] **CU Consumption Breakdown (for both V0 & V1):**
            *   Isolate CU cost of: PDA derivations, Merkle verification, `token::transfer` CPI, `ClaimReceipt` initialization.
        *   [ ] **Transaction Size (for both V0 & V1):**
            *   Measure serialized transaction size. Determine max proof length for V0 and effective size for V1.
        *   [ ] **Maximum Practical Cohort Size (for both V0 & V1):**
            *   Determine if mechanisms hit CU or transaction size limits for very large cohorts.
    *   **Account Sizes & Rent:**
        *   [ ] Document rent cost for `Campaign`, `ClaimReceipt` PDAs.
        *   [ ] Document and compare rent cost for `CohortV0` vs `CohortV1` (due to trunk storage).
    *   **Initialization Instructions (`initialize_campaign`, `initialize_cohort_v0`, `initialize_cohort_v1`):**
        *   [ ] CU consumption for each.
        *   [ ] Transaction sizes for each.
*   **`prism-cli` Benchmarking Areas (if performance becomes a concern for very large datasets):**
    *   [ ] Time to generate Merkle roots/trunks for large claimant lists (V0 vs V1 data generation).
    *   [ ] Time to generate proofs for all claimants (V0 vs V1 proofs).
    *   [ ] Memory usage during processing.

## 5. Documentation Checklist

*   [ ] `README.md`: Review and update as features are finalized and `prism-cli` is built.
*   [ ] `DESIGN_NOTES.md`: Sync instruction arguments, account fields, and operational flows with final on-chain implementation, including V0/V1 distinctions.
*   [ ] `CAMPAIGN_SETUP_GUIDE.md`: Review and update to accurately reflect `prism-cli` commands and outputs, including V0/V1 options if exposed to users.
*   [ ] `PROJECT_PLAN.md` (This document): Keep updated with progress.
*   [ ] **API/SDK Documentation (Future):** Plan if a client-side SDK is developed.

## 6. Future Milestones (Post-MVP & Benchmarking)

*   [ ] Client-side SDK (JavaScript/TypeScript) development.
*   [ ] Advanced `prism-cli` features (e.g., campaign management, interactive modes).
*   [ ] Preparation for Security Audit.
*   [ ] Decision on preferred Cohort/Claim version (V0, V1, or support both) based on benchmarks. 
# Prism Protocol: Project Plan & Checklist

## 1. Core Project Goal

To enable efficient, scalable, and verifiable token distribution on Solana, minimizing write contention and optimizing on-chain resources. (See `README.md` for full details).

## 2. Core Components - Implementation Checklist

### On-Chain Program (`programs/prism-protocol/src/`)

*   **State Accounts (`state.rs`):**
    *   [x] `CampaignV0` struct defined ✅
    *   [x] `CohortV0` struct defined ✅
    *   [x] `ClaimReceiptV0` struct defined ✅
    *   [ ] Future enhanced cohort versions (e.g., with optimizations) - *Future Design*
*   **Instructions (`instructions/` & `lib.rs`):**
    *   [x] `handle_initialize_campaign_v0` ✅
    *   [x] `handle_initialize_cohort_v0` ✅
    *   [x] `handle_claim_tokens_v0` ✅
    *   [x] `handle_set_campaign_active_status` ✅
    *   [x] `handle_reclaim_tokens` ✅
    *   [ ] Future enhanced instruction versions - *Future Design*
*   **Merkle Logic:**
    *   [x] `ClaimLeaf` struct and `hash_claim_leaf` function (`merkle_leaf.rs`) ✅
    *   [x] `verify_merkle_proof` function (in `claim_tokens_v0.rs`) ✅
    *   [x] Domain separation with 0x00/0x01 prefixes for security ✅
*   **Program Entrypoint (`lib.rs`):**
    *   [x] Declare program ID ✅
    *   [x] Define `initialize_campaign_v0` public instruction ✅
    *   [x] Define `initialize_cohort_v0` public instruction ✅
    *   [x] Define `claim_tokens_v0` public instruction ✅
    *   [x] Define `set_campaign_active_status` public instruction ✅
    *   [x] Define `reclaim_tokens` public instruction ✅

### Crate Structure (Completed Refactoring)

*   **Core Program (`prism-protocol`):**
    *   [x] Minimal on-chain program with core functionality ✅
    *   [x] Clean separation from off-chain utilities ✅
*   **SDK Crate (`prism-protocol-sdk`):**
    *   [x] Address finders for PDA derivation ✅
    *   [x] Instruction builders for transaction construction ✅
    *   [x] Client-side utilities ✅
*   **Merkle Tree Crate (`prism-protocol-merkle`):**
    *   [x] Off-chain merkle tree construction ✅
    *   [x] Proof generation and verification utilities ✅
    *   [x] Consistent hashing for vault assignment ✅
    *   [x] Custom hasher with domain separation ✅
*   **Testing Utilities (`prism-protocol-testing`):**
    *   [x] Shared test fixtures and utilities ✅
    *   [x] Mollusk SVM integration helpers ✅

### Off-Chain CLI (`apps/prism-cli`)

*   **Status:** *To Be Designed & Implemented*
*   **Core Functionality Checklist:**
    *   [ ] Campaign configuration file parsing
    *   [ ] Claimant list processing (CSV, JSON, etc.)
    *   [ ] `ClaimLeaf` data generation (claimant, assigned_vault, entitlements)
    *   [ ] Merkle tree generation for each cohort
    *   [ ] `campaign_fingerprint` calculation (from sorted cohort merkle roots)
    *   [ ] Individual Merkle proof generation for each claimant
    *   [ ] Output: Parameters for on-chain instructions
    *   [ ] Output: Vault funding requirements report
    *   [ ] Output: Claimant lookup files (proofs, assigned_vaults, entitlements, etc.)

### Testing (using Mollusk SVM)

*   **Unit Tests (On-Chain):**
    *   [x] `merkle_leaf.rs` tests for `hash_claim_leaf` ✅
    *   [x] Merkle tree construction and proof generation tests ✅
    *   [x] Consistent hashing tests ✅
*   **Integration Tests (using Mollusk SVM):**
    *   [x] Full `initialize_campaign_v0` instruction test ✅
    *   [x] Full `initialize_cohort_v0` instruction test ✅
    *   [x] Full `claim_tokens_v0` instruction test (various scenarios) ✅
    *   [x] Full `set_campaign_active_status` instruction test ✅
    *   [x] Full `reclaim_tokens` instruction test ✅
    *   [x] End-to-end test: Campaign setup → cohort setup → successful claim ✅
    *   [x] Merkle proof generation and verification tests ✅
    *   [x] Instruction building tests ✅

## 3. Key Design Decisions & Implementation Notes

*   **✅ Campaign Fingerprint System:**
    *   Campaigns are identified by a cryptographic fingerprint derived from constituent cohort merkle roots
    *   Ensures immutability and verifiability of campaign definitions
*   **✅ Merkle Tree Security:**
    *   Domain separation using 0x00 prefix for leaves, 0x01 for internal nodes
    *   Prevents second preimage attacks and ensures proof integrity
*   **✅ Vault Assignment:**
    *   Consistent hashing distributes claimants across multiple vaults
    *   Reduces write contention while maintaining deterministic assignment
*   **✅ Modular Architecture:**
    *   Clean separation between on-chain program and off-chain utilities
    *   Reusable SDK and testing components

## 4. Benchmarking Plan (using Mollusk SVM)

*   **Objective:** Quantitatively validate performance, scalability, and resource consumption.
*   **On-Chain Benchmarking Areas:**
    *   [ ] **`claim_tokens_v0` Performance:**
        *   CU consumption vs. proof length for various cohort sizes
        *   Transaction size analysis
        *   Maximum practical cohort size determination
    *   [ ] **Account Sizes & Rent:**
        *   Document rent costs for `CampaignV0`, `CohortV0`, `ClaimReceiptV0` PDAs
        *   Compare costs across different vault configurations
    *   [ ] **Initialization Instructions:**
        *   CU consumption for each instruction type
        *   Transaction size analysis
*   **Off-Chain Benchmarking Areas:**
    *   [ ] Merkle tree generation time for large claimant lists
    *   [ ] Proof generation time and memory usage
    *   [ ] Consistent hashing performance

## 5. Documentation Checklist

*   [x] `PROJECT_PLAN.md`: Updated to reflect current implementation ✅
*   [ ] `DESIGN_NOTES.md`: Update to match actual implementation
*   [ ] `CAMPAIGN_SETUP_GUIDE.md`: Update for future CLI implementation
*   [ ] `README.md`: Update with new crate structure information
*   [ ] **API Documentation:** Document the SDK crate public interfaces

## 6. Future Milestones (Post-MVP)

*   [ ] Complete CLI tool implementation
*   [ ] Performance benchmarking and optimization
*   [ ] Enhanced cohort versions with additional optimizations
*   [ ] Client-side SDK (JavaScript/TypeScript) development
*   [ ] Security audit preparation
*   [ ] Advanced CLI features (campaign management, interactive modes)

## 7. Current Status Summary

**✅ Completed:**
- Core on-chain program with all essential instructions
- Complete crate separation and modular architecture
- Comprehensive test suite with Mollusk SVM integration
- Merkle tree utilities with security best practices
- SDK for client-side transaction building

**🚧 In Progress:**
- Documentation updates to match implementation

**📋 Next Phase:**
- CLI tool development
- Performance benchmarking
- Enhanced features and optimizations 
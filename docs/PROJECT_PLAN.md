# Prism Protocol: Project Plan & Checklist

## 1. Core Project Goal

To enable efficient, scalable, and verifiable token distribution on Solana, minimizing write contention and optimizing on-chain resources. (See `README.md` for full details).

## 2. Core Components - Implementation Checklist

### On-Chain Program (`programs/prism-protocol/src/`)

- **State Accounts (`state.rs`):**
  - [x] `CampaignV0` struct defined ✅
  - [x] `CohortV0` struct defined ✅
  - [x] `ClaimReceiptV0` struct defined ✅
  - [ ] Future enhanced cohort versions (e.g., with optimizations) - _Future Design_
- **Instructions (`instructions/` & `lib.rs`):**
  - [x] `handle_initialize_campaign_v0` ✅
  - [x] `handle_initialize_cohort_v0` ✅
  - [x] `handle_claim_tokens_v0` ✅
  - [x] `handle_set_campaign_active_status` ✅
  - [x] `handle_reclaim_tokens` ✅
  - [ ] Future enhanced instruction versions - _Future Design_
- **Merkle Logic:**
  - [x] `ClaimLeaf` struct and `hash_claim_leaf` function (`merkle_leaf.rs`) ✅
  - [x] `verify_merkle_proof` function (in `claim_tokens_v0.rs`) ✅
  - [x] Domain separation with 0x00/0x01 prefixes for security ✅
- **Program Entrypoint (`lib.rs`):**
  - [x] Declare program ID ✅
  - [x] Define `initialize_campaign_v0` public instruction ✅
  - [x] Define `initialize_cohort_v0` public instruction ✅
  - [x] Define `claim_tokens_v0` public instruction ✅
  - [x] Define `set_campaign_active_status` public instruction ✅
  - [x] Define `reclaim_tokens` public instruction ✅

### Crate Structure (Completed Refactoring)

- **Core Program (`prism-protocol`):**
  - [x] Minimal on-chain program with core functionality ✅
  - [x] Clean separation from off-chain utilities ✅
- **SDK Crate (`prism-protocol-sdk`):**
  - [x] Address finders for PDA derivation ✅
  - [x] Instruction builders for transaction construction ✅
  - [x] Client-side utilities ✅
- **Merkle Tree Crate (`prism-protocol-merkle`):**
  - [x] Off-chain merkle tree construction ✅
  - [x] Proof generation and verification utilities ✅
  - [x] Consistent hashing for vault assignment ✅
  - [x] Custom hasher with domain separation ✅
- **Testing Utilities (`prism-protocol-testing`):**
  - [x] Shared test fixtures and utilities ✅
  - [x] Mollusk SVM integration helpers ✅

### Off-Chain CLI (`apps/prism-cli`)

- **Status:** _To Be Designed & Implemented_
- **Priority:** High - Critical next step for end-to-end usability

#### Planned CLI Features & Implementation Phases

**Phase 0: Fixture Generation (For Benchmarking)**

- **Purpose:** Generate large-scale test datasets for performance validation
- **Commands:**
  - `prism-cli generate-fixtures --count <N> --seed <SEED> [options]`
  - Support for deterministic pubkey generation (no real keypairs needed)
  - Configurable entitlement distributions (uniform, realistic, custom ranges)
  - Multiple output formats (CSV, JSON)
  - Multi-cohort fixture generation
- **Key Features:**
  - Deterministic generation for reproducible benchmarks
  - Memory-efficient for millions of claimants
  - Realistic distribution patterns for testing

**Phase 1: Core Campaign Generation**

- **Purpose:** Process campaign configs and generate all necessary data
- **Commands:**
  - `prism-cli generate-campaign <config.yaml>`
- **Input:** Campaign configuration file defining cohorts and claimants
- **Output:**
  - Campaign fingerprint calculation
  - Individual cohort merkle roots
  - Vault funding requirements report
  - Claimant lookup files with proofs and assignments
  - On-chain initialization parameters

**Phase 2: Transaction Building & Deployment**

- **Purpose:** Actually deploy campaigns on-chain
- **Commands:**
  - `prism-cli deploy-campaign --config <config.yaml> --keypair <admin.json>`
  - `prism-cli deploy-cohort --campaign <fingerprint> --cohort <merkle-root> --keypair <admin.json>`
- **Features:**
  - Automatic transaction construction using SDK utilities
  - Vault funding validation
  - Deployment status tracking

**Phase 3: Campaign Management**

- **Purpose:** Administrative operations for live campaigns
- **Commands:**
  - `prism-cli pause-campaign <fingerprint> --keypair <admin.json>`
  - `prism-cli resume-campaign <fingerprint> --keypair <admin.json>`
  - `prism-cli reclaim-tokens <fingerprint> <cohort-root> --keypair <admin.json>`
  - `prism-cli campaign-status <fingerprint>`
- **Features:**
  - Campaign lifecycle management
  - Token recovery after distribution periods
  - Status monitoring and reporting

**Phase 4: Advanced Features (Future)**

- **Purpose:** Enhanced functionality for complex use cases
- **Potential Commands:**
  - `prism-cli validate-campaign <config.yaml>` (dry-run validation)
  - `prism-cli estimate-costs <config.yaml>` (rent and transaction cost estimation)
  - `prism-cli export-proofs <fingerprint> --format <json|api>` (proof serving formats)
  - `prism-cli benchmark <config.yaml>` (performance testing)

#### Core Functionality Checklist (Detailed)

- **Configuration Processing:**

  - [ ] YAML/JSON campaign configuration parsing
  - [ ] Claimant list processing (CSV, JSON, multiple formats)
  - [ ] Input validation and error handling
  - [ ] Configuration schema documentation

- **Merkle Tree Operations:**

  - [ ] `ClaimLeaf` data generation (claimant, assigned_vault, entitlements)
  - [ ] Merkle tree generation for each cohort using `prism-protocol-merkle`
  - [ ] Consistent hashing for vault assignment
  - [ ] Individual proof generation for all claimants

- **Campaign Fingerprint System:**

  - [ ] Cohort merkle root collection and sorting
  - [ ] Deterministic `campaign_fingerprint` calculation
  - [ ] Fingerprint validation and collision detection

- **Output Generation:**

  - [ ] On-chain initialization parameters (ready-to-use)
  - [ ] Vault funding requirements with exact amounts
  - [ ] Claimant lookup files (proofs, assigned_vaults, entitlements)
  - [ ] Human-readable reports and summaries

- **Integration & Testing:**
  - [ ] Integration with `prism-protocol-sdk` for transaction building
  - [ ] Integration with `prism-protocol-merkle` for tree operations
  - [ ] Comprehensive test suite with fixture generation
  - [ ] Performance benchmarking with large datasets

#### Technical Implementation Notes

- **Dependencies:** Will use existing crates (`prism-protocol-sdk`, `prism-protocol-merkle`)
- **Configuration Format:** YAML for human readability, with JSON support
- **Performance Targets:** Handle millions of claimants efficiently
- **Error Handling:** Comprehensive validation with helpful error messages
- **Output Formats:** Multiple formats to support different integration needs

### Testing (using Mollusk SVM)

- **Unit Tests (On-Chain):**
  - [x] `merkle_leaf.rs` tests for `hash_claim_leaf` ✅
  - [x] Merkle tree construction and proof generation tests ✅
  - [x] Consistent hashing tests ✅
- **Integration Tests (using Mollusk SVM):**
  - [x] Full `initialize_campaign_v0` instruction test ✅
  - [x] Full `initialize_cohort_v0` instruction test ✅
  - [x] Full `claim_tokens_v0` instruction test (various scenarios) ✅
  - [x] Full `set_campaign_active_status` instruction test ✅
  - [x] Full `reclaim_tokens` instruction test ✅
  - [x] End-to-end test: Campaign setup → cohort setup → successful claim ✅
  - [x] Merkle proof generation and verification tests ✅
  - [x] Instruction building tests ✅

## 3. Key Design Decisions & Implementation Notes

- **✅ Campaign Fingerprint System:**
  - Campaigns are identified by a cryptographic fingerprint derived from constituent cohort merkle roots
  - Ensures immutability and verifiability of campaign definitions
- **✅ Merkle Tree Security:**
  - Domain separation using 0x00 prefix for leaves, 0x01 for internal nodes
  - Prevents second preimage attacks and ensures proof integrity
- **✅ Vault Assignment:**
  - Consistent hashing distributes claimants across multiple vaults
  - Reduces write contention while maintaining deterministic assignment
- **✅ Modular Architecture:**
  - Clean separation between on-chain program and off-chain utilities
  - Reusable SDK and testing components

## 4. Benchmarking Plan (using Mollusk SVM)

- **Objective:** Quantitatively validate performance, scalability, and resource consumption.
- **On-Chain Benchmarking Areas:**
  - [ ] **`claim_tokens_v0` Performance:**
    - CU consumption vs. proof length for various cohort sizes
    - Transaction size analysis
    - Maximum practical cohort size determination
  - [ ] **Account Sizes & Rent:**
    - Document rent costs for `CampaignV0`, `CohortV0`, `ClaimReceiptV0` PDAs
    - Compare costs across different vault configurations
  - [ ] **Initialization Instructions:**
    - CU consumption for each instruction type
    - Transaction size analysis
- **Off-Chain Benchmarking Areas:**
  - [ ] Merkle tree generation time for large claimant lists
  - [ ] Proof generation time and memory usage
  - [ ] Consistent hashing performance

## 5. Documentation Checklist

- [x] `PROJECT_PLAN.md`: Updated to reflect current implementation ✅
- [ ] `DESIGN_NOTES.md`: Update to match actual implementation
- [ ] `CAMPAIGN_SETUP_GUIDE.md`: Update for future CLI implementation
- [ ] `README.md`: Update with new crate structure information
- [ ] **API Documentation:** Document the SDK crate public interfaces

## 6. Future Milestones (Post-MVP)

- [ ] Complete CLI tool implementation
- [ ] Performance benchmarking and optimization
- [ ] Enhanced cohort versions with additional optimizations
- [ ] Client-side SDK (JavaScript/TypeScript) development
- [ ] Security audit preparation
- [ ] Advanced CLI features (campaign management, interactive modes)

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

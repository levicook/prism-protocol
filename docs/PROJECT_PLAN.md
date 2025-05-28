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

### Off-Chain CLI (`apps/prism-protocol-cli`)

- **Status:** _Phase 0 & 1 Implemented, Phase 2+ In Progress_
- **Priority:** High - Core functionality complete, deployment features next

#### Planned CLI Features & Implementation Phases

**Phase 0: Fixture Generation (For Benchmarking) ✅ COMPLETED**

- **Purpose:** Generate large-scale test datasets for performance validation
- **Commands:**
  - `cargo run -p prism-protocol-cli -- generate-fixtures --count <N> --seed <SEED> [options]`
  - Support for deterministic pubkey generation (no real keypairs needed)
  - Configurable entitlement distributions (uniform, realistic, exponential)
  - CSV output format (campaign.csv and cohorts.csv)
  - Multi-cohort fixture generation with configurable cohort counts
- **Key Features:**
  - ✅ Deterministic generation for reproducible benchmarks
  - ✅ Memory-efficient for millions of claimants
  - ✅ Realistic distribution patterns for testing
  - ✅ Progress tracking for large datasets
  - ✅ Configurable cohort and entitlement ranges

**Phase 1: Core Campaign Generation ✅ COMPLETED**

- **Purpose:** Process campaign configs and generate all necessary data
- **Commands:**
  - `cargo run -p prism-protocol-cli -- compile-campaign --campaign-csv <file> --cohorts-csv <file> --mint <pubkey> --admin-keypair <file>`
- **Input:** Two CSV files (campaign claimants and cohort configuration)
- **Output:**
  - ✅ SQLite database with complete campaign structure
  - ✅ Vault count calculation and funding requirements
  - ✅ Claimant records with entitlements and vault assignments
  - ✅ Admin keypair validation and public key storage
  - ✅ Campaign fingerprint calculation from sorted cohort merkle roots
  - ✅ Individual cohort merkle roots generated and stored
  - ✅ Merkle proofs for all claimants generated and stored
  - ✅ Complete merkle tree integration with consistent hashing
  - ✅ Hex-encoded storage of all merkle data in database

**Phase 2: Transaction Building & Deployment 🚧 IN PROGRESS**

- **Purpose:** Actually deploy campaigns on-chain
- **Commands:**
  - `cargo run -p prism-protocol-cli -- deploy-campaign --config <config.yaml> --admin-keypair <admin.json>`
  - `cargo run -p prism-protocol-cli -- deploy-cohort --campaign <fingerprint> --cohort <merkle-root> --admin-keypair <admin.json>`
- **Features:**
  - 🚧 Automatic transaction construction using SDK utilities
  - 🚧 Vault funding validation
  - 🚧 Deployment status tracking
  - 🚧 Merkle tree generation and fingerprint calculation
  - 🚧 Integration with prism-protocol-merkle crate

**Phase 3: Campaign Management 📋 PLANNED**

- **Purpose:** Administrative operations for live campaigns
- **Commands:**
  - `cargo run -p prism-protocol-cli -- pause-campaign <fingerprint> --admin-keypair <admin.json>`
  - `cargo run -p prism-protocol-cli -- resume-campaign <fingerprint> --admin-keypair <admin.json>`
  - `cargo run -p prism-protocol-cli -- reclaim-tokens <fingerprint> <cohort-root> --admin-keypair <admin.json>`
  - `cargo run -p prism-protocol-cli -- campaign-status <fingerprint>`
- **Features:**
  - 📋 Campaign lifecycle management
  - 📋 Token recovery after distribution periods
  - 📋 Status monitoring and reporting

**Phase 4: Advanced Features (Future)**

- **Purpose:** Enhanced functionality for complex use cases
- **Potential Commands:**
  - `prism-protocol-cli validate-campaign <config.yaml>` (dry-run validation)
  - `prism-protocol-cli estimate-costs <config.yaml>` (rent and transaction cost estimation)
  - `prism-protocol-cli export-proofs <fingerprint> --format <json|api>` (proof serving formats)
  - `prism-protocol-cli benchmark <config.yaml>` (performance testing)

#### Core Functionality Checklist (Detailed)

- **Configuration Processing:**

  - [x] CSV campaign configuration parsing ✅
  - [x] Claimant list processing (CSV format) ✅
  - [x] Input validation and error handling ✅
  - [x] Cohort consistency validation ✅
  - [ ] YAML/JSON configuration support (future enhancement)
  - [ ] Configuration schema documentation

- **Database & Storage:**

  - [x] SQLite database schema design ✅
  - [x] Campaign metadata storage ✅
  - [x] Cohort and claimant data storage ✅
  - [x] Vault funding requirements calculation ✅
  - [x] Admin keypair validation and public key storage ✅

- **Merkle Tree Operations:**

  - [x] `ClaimLeaf` data generation (claimant, assigned_vault, entitlements) ✅
  - [x] Merkle tree generation for each cohort using `prism-protocol-merkle` ✅
  - [x] Consistent hashing for vault assignment ✅
  - [x] Individual proof generation for all claimants ✅

- **Campaign Fingerprint System:**

  - [x] Cohort merkle root collection and sorting ✅
  - [x] Deterministic `campaign_fingerprint` calculation ✅
  - [x] Fingerprint validation and collision detection ✅

- **Output Generation:**

  - [x] SQLite database with complete campaign structure ✅
  - [x] Vault funding requirements with exact amounts ✅
  - [x] Claimant records with entitlements ✅
  - [x] Merkle proofs and vault assignments ✅
  - [x] Campaign fingerprint and cohort merkle roots ✅
  - [ ] On-chain initialization parameters (ready-to-use)
  - [ ] Human-readable reports and summaries

- **Integration & Testing:**
  - [x] Comprehensive test suite with fixture generation ✅
  - [x] CSV parsing and validation tests ✅
  - [x] Database creation and population tests ✅
  - [x] CLI integration test automation with real command execution ✅
  - [x] Performance benchmarking test suite ✅
  - [x] Integration with `prism-protocol-merkle` for tree operations ✅
  - [x] Deterministic behavior validation and error handling tests ✅
  - [ ] Integration with `prism-protocol-sdk` for transaction building
  - [ ] Performance benchmarking with large datasets (automated via make test-performance)

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
- **CLI Phase 0**: Fixture generation with deterministic test data
- **CLI Phase 1**: Complete campaign compilation with full merkle tree integration
  - CSV processing and database creation
  - Merkle tree generation with consistent hashing
  - Campaign fingerprint calculation from sorted cohort roots
  - Individual merkle proofs for all claimants
  - Hex-encoded storage of all merkle data
- **Test Automation System**: Comprehensive CLI testing with real command execution
  - CLI integration tests with database validation
  - Performance benchmarking suite
  - Automated test orchestration via Makefile
  - Organized test artifact management
- Admin keypair validation and proper public key storage
- Vault count calculation and funding requirements

**🚧 In Progress:**

- **CLI Phase 2**: On-chain deployment functionality
- Transaction building using prism-protocol-sdk
- Campaign and cohort deployment commands

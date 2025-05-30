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
  - [x] `handle_create_vault_v0` ✅
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
  - [x] Define `create_vault_v0` public instruction ✅
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

- **Status:** _Phase 0, 1, 2 Completed, Phase 3 Partially Implemented_
- **Priority:** High - Core functionality complete, claiming ecosystem and architecture improvements next

#### Planned CLI Features & Implementation Phases

**Phase 0: Enhanced Fixture Generation (For Testing) ✅ COMPLETED**

- **Purpose:** Generate organized test datasets with real keypairs for development and testing
- **Commands:**
  - `cargo run -p prism-protocol-cli -- generate-fixtures --campaign-name <NAME> [options]`
  - Organized directory structure: `test-artifacts/fixtures/{campaign-slug}/`
  - Real Solana keypair generation for all claimants (no more dummy pubkeys)
  - Individual keypair files with complete metadata for each claimant
  - CSV output format (campaign.csv and cohorts.csv)
  - Multi-cohort fixture generation with configurable cohort counts
- **Key Features:**
  - ✅ Organized campaign-specific directory structure
  - ✅ Real keypair generation for authentic testing
  - ✅ Individual keypair file storage with metadata
  - ✅ Overwrite protection to prevent data loss
  - ✅ Multiple distribution patterns (uniform, realistic, exponential)
  - ✅ Progress tracking for large datasets
  - ✅ Configurable cohort and entitlement ranges
  - ✅ Reproducible benchmarking via fixture archiving (replaces deterministic seeds)

**Phase 1: Core Infrastructure ✅ **COMPLETED\*\*

**Target: Week 1-2 of Sprint**

### Campaign Management ✅

- [x] Campaign compilation from CSV sources → SQLite database
- [x] Campaign deployment with comprehensive on-chain state management
- [x] Campaign status querying and verification
- [x] Automated vault creation and funding
- [x] Campaign activation controls

### Token Distribution ✅

- [x] **END-TO-END TOKEN CLAIMING WORKING** 🎉
- [x] Merkle proof verification and validation
- [x] Multi-cohort support with deterministic vault assignment
- [x] Comprehensive claim validation and double-spend protection
- [x] Automatic token account creation for claimants
- [x] **CRITICAL BUG FIXED**: Vault address derivation now uses correct campaign fingerprint

### Test Infrastructure ✅

- [x] Enhanced fixture generation with real keypairs and organized directory structure
- [x] Deterministic address derivation across compilation and deployment
- [x] Campaign database schema with complete merkle tree integration
- [x] **Clean fixture organization**: `test-artifacts/fixtures/{campaign-slug}/`

**Status: ✅ PHASE 1 COMPLETE - Core claiming functionality fully operational**

**Phase 2: Enhanced Command Interface ✅ **COMPLETED\*\*

**Target: Week 2-3 of Sprint**

### CLI Command Suite ✅

- [x] Enhanced `generate-fixtures` with real keypair generation
- [x] `compile-campaign` with corrected address derivation logic
- [x] `deploy-campaign` with comprehensive deployment verification
- [x] `campaign-status` with accurate on-chain state reporting
- [x] `claim-tokens` with **working end-to-end token claiming**

### Error Handling & Validation ✅

- [x] Comprehensive input validation and user-friendly error messages
- [x] Pre-flight checks for SOL balances, token accounts, and RPC connectivity
- [x] Proper error handling for insufficient funds and network issues
- [x] **Critical bug detection and resolution** for address derivation mismatches

**Status: ✅ PHASE 2 COMPLETE - All core CLI commands operational**

**Phase 3: Claiming Ecosystem Foundation 🚧 PARTIALLY IMPLEMENTED**

- **Purpose:** Build complete claiming infrastructure and query tools
- **Strategic Approach:** Database-first with blockchain verification for comprehensive claim management

- **Implemented Commands:**

  - ✅ `cargo run -p prism-protocol-cli -- check-eligibility <pubkey_or_keypair_file> --campaign-db <db> [--rpc-url <url>]`
    - Auto-detects pubkey string vs keypair file path
    - Shows eligibility across all cohorts with entitlements and vault assignments
    - **Hybrid verification**: Database query + on-chain claim receipt checking
    - Proper token amount formatting with actual mint decimals
    - Detects database vs blockchain mismatches
  - ✅ `cargo run -p prism-protocol-cli -- query-claims <pubkey_or_keypair_file> [--campaign-fingerprint <fp>] [--rpc-url <url>]`
    - Blockchain-first approach using getProgramAccounts
    - Campaign filtering to avoid "query the world" problem
    - Pure on-chain claim history with transaction signatures and timestamps
    - Consistent interface with auto-detection of input format

- **Infrastructure Components:**

  1. **Secrets Management System** ✅ COMPLETED

     - Team-based keypair encryption/decryption with age encryption
     - `scripts/encrypt-secrets` and `scripts/decrypt-secrets`
     - Public key management in `secrets/recipients.txt`
     - Gitignore protection for decrypted keypairs

  2. **CLI Configuration Management** ✅ COMPLETED

     - `scripts/generate-configs` for organized Solana CLI configs
     - Multi-network support (localnet, devnet, mainnet excluded for safety)
     - Automatic config generation from encrypted keypairs
     - Proper RPC URL and commitment level configuration
     - Directory structure: `test-artifacts/configs/{network}/{keypair}.yml`

  3. **End-to-End Testing Infrastructure** ✅ COMPLETED
     - Complete test validator → deployment → funding → verification workflow
     - Real WSOL wrapping and token operations
     - Campaign deployment with actual token transfers
     - Database vs blockchain consistency validation

- **Remaining Components (Not Yet Implemented):**
  - [ ] **Enhanced Fixture Generator** - Campaign-organized directory structure
  - [ ] **API Server** (`prism-protocol-cli serve-api`) - Proof serving infrastructure
  - [ ] **CLI Claim Command** - `claim-tokens` using API server for proof lookup
  - [ ] **dApp Frontend** - User interface for claiming

**Phase 4: Campaign Management & Production Readiness 📋 PLANNED**

- **Purpose:** Administrative operations, production deployment, and campaign creation tools
- **Strategic Components:**

  1. **Campaign Admin dApp** (New Strategic Component)

     - Web UI for campaign operators to define campaigns (replaces manual CSV creation)
     - Visual cohort configuration and claimant list management
     - Export to CLI-compatible formats
     - Campaign preview and validation
     - Integration with secrets management for secure admin operations

  2. **CLI Administrative Operations**

     - `cargo run -p prism-protocol-cli -- pause-campaign <fingerprint> --admin-keypair <admin.json>`
     - `cargo run -p prism-protocol-cli -- resume-campaign <fingerprint> --admin-keypair <admin.json>`
     - `cargo run -p prism-protocol-cli -- reclaim-tokens <fingerprint> <cohort-root> --admin-keypair <admin.json>`

  3. **Production Infrastructure**
     - Docker containerization for full stack
     - API rate limiting and security
     - Performance optimization for 100K+ claimants
     - Monitoring and alerting

**Phase 5: Advanced Features (Future)**

- **Purpose:** Enhanced functionality for complex use cases
- **Architectural Improvements:**
  - **Robust Deployment State Management** - Implement proper state validation before campaign activation
  - **Multi-Admin Coordination** - Support for distributed campaign deployment
  - **Enhanced Error Recovery** - Rollback and retry mechanisms for failed deployments
- **Potential Commands:**
  - `prism-protocol-cli validate-campaign <config.yaml>` (dry-run validation)
  - `prism-protocol-cli estimate-costs <config.yaml>` (rent and transaction cost estimation)
  - `prism-protocol-cli export-proofs <fingerprint> --format <json|api>` (proof serving formats)
  - `prism-protocol-cli benchmark <config.yaml>` (performance testing)
- **Advanced Features:**
  - Jito bundle building for MEV protection
  - Claim status tracking and analytics
  - Horizontal scaling support
  - Campaign templates and batch operations

## 5. Testing Infrastructure & Coverage Analysis

### Current Test Coverage ✅

**Unit Tests (`cargo test`):**

- [x] Merkle tree generation and proof verification ✅
- [x] Address derivation functions ✅
- [x] Instruction builders ✅
- [x] Basic CLI argument parsing ✅
- [x] Token decimal formatting and mint account parsing ✅

**CLI Integration Tests (`scripts/test-cli.sh`):**

- [x] Fixture generation with various distributions ✅
- [x] Campaign compilation and database creation ✅
- [x] Database schema validation ✅
- [x] Merkle tree storage and retrieval ✅

**Anchor Program Tests (`anchor test`):**

- [x] Campaign initialization ✅
- [x] Cohort initialization ✅
- [x] Vault creation ✅
- [x] Token claiming with merkle proofs ✅
- [x] Access control and authorization ✅

**End-to-End Tests (Manual & Automated):** ✅ SIGNIFICANTLY ADVANCED

- [x] Complete workflow: fixture generation → campaign compilation → deployment ✅
- [x] Local validator setup and SPL token operations ✅
- [x] Campaign and cohort PDA deployment with real transactions ✅
- [x] Vault creation and funding with WSOL token transfers ✅
- [x] Database tracking of deployment signatures ✅
- [x] Progressive deployment status updates ✅
- [x] **Token Decimal Safety Testing**: Verified proper WSOL (9 decimals) handling ✅
- [x] **Idempotent Deployment Testing**: Re-running deploy command safely skips completed steps ✅
- [x] **Secrets Management Integration**: Full encryption/decryption workflow with team keypairs ✅
- [x] **CLI Configuration Testing**: Multi-network config generation and usage ✅
- [x] **Eligibility Checking**: Database + blockchain verification with real deployed campaigns ✅
- [x] **Claims Query Testing**: Blockchain-first claim history retrieval ✅

### Enhanced Database Schema ✅

**Vault Lifecycle Tracking:**

- [x] `created_at` - timestamp when vault PDA was created on-chain ✅
- [x] `created_by_tx` - transaction signature for vault creation ✅
- [x] `funded_at` - timestamp when vault was funded with tokens ✅
- [x] `funded_by_tx` - transaction signature for vault funding ✅
- [x] Surgical database updates for each operation ✅

### Critical Achievements & Bug Fixes ✅

**Token Decimal Safety (CRITICAL):**

- [x] **Bug Fix**: Replaced hardcoded 9-decimal assumption with proper mint account fetching ✅
- [x] **Impact**: Prevents 1000x overfunding disasters with tokens like USDC (6 decimals) ✅
- [x] **Implementation**: `get_mint_decimals()` and `format_token_amount()` functions ✅
- [x] **Testing**: Verified correct behavior with WSOL (9 decimals) in production deployment ✅

**Idempotent Deployment (CRITICAL):**

- [x] **Smart Pre-flight Checks**: Calculate actual tokens needed vs database totals ✅
- [x] **Safe Re-runs**: Deploy command can be run multiple times without overfunding ✅
- [x] **Vault Balance Checking**: `fund_vault_if_needed()` only transfers difference ✅
- [x] **Comprehensive Reporting**: Shows "Actually needed: 0 tokens" when vaults already funded ✅

### Identified Testing Gaps ⚠️

**Architecture & State Management:**

- [ ] **Deployment State Validation** - Testing proper campaign activation prerequisites
- [ ] **State Machine Testing** - Campaign/cohort/vault state transitions
- [ ] **Multi-Admin Scenarios** - Testing distributed deployment coordination
- [ ] **Error Recovery** - Testing deployment rollback and retry mechanisms

**Claiming Ecosystem Gaps:**

- [ ] **Enhanced Fixture Generator** - Campaign-organized directory structure with HD wallets
- [ ] **API Server** - No proof serving infrastructure exists yet
- [ ] **CLI Claim Command** - No `claim-tokens` command implemented yet
- [ ] **Actual Token Claiming** - End-to-end claim execution and receipt verification

**Missing End-to-End Scenarios:**

- [ ] **Token claiming workflow** - Real claim transactions with proof verification
- [ ] **Multi-user claim scenarios** - Testing concurrent claims with real keypairs
- [ ] **Cross-network testing** - Devnet deployment and operation
- [ ] **Large-scale testing** - Testing with realistic token amounts and user counts (100K+ claimants)
- [ ] **Different Token Types** - Testing with USDC, other SPL tokens (decimal validation)

**Performance & Scale Testing:**

- [ ] **Large merkle trees** - Testing with 100K+ claimants
- [ ] **API server performance** - Once implemented, response times with large databases
- [ ] **Transaction batching** - Optimizing deployment transaction costs
- [ ] **Memory usage profiling** - Ensuring efficient resource usage with large datasets

### Current Status & Next Steps 🎯

**✅ Completed:**

- Core on-chain program with all essential instructions including vault creation
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
- **CLI Phase 2**: Complete on-chain deployment functionality with critical safety improvements
  - Campaign and cohort PDA deployment
  - Vault creation and funding with token transfers
  - **Token Decimal Safety**: Proper mint account fetching prevents overfunding disasters
  - **Idempotent Deployment**: Smart pre-flight checks and safe re-runs
  - Progressive database updates with transaction signatures
  - Pre-flight checks and comprehensive verification
- **CLI Phase 3 (Partial)**: Query and eligibility infrastructure
  - `check-eligibility` command with database + blockchain verification
  - `query-claims` command with blockchain-first approach
  - Proper token amount formatting with actual mint decimals
  - Auto-detection of pubkey vs keypair input formats
- **Secrets Management System**: Team-based keypair encryption/decryption
  - Age encryption with public key management
  - Gitignore protection for decrypted keypairs
  - `scripts/encrypt-secrets` and `scripts/decrypt-secrets`
- **CLI Configuration Management**: Organized Solana CLI configs
  - Multi-network support (localnet, devnet)
  - Automatic config generation from encrypted keypairs
  - `scripts/generate-configs` for clean configuration management
- **End-to-End Testing Infrastructure**: Complete deployment validation
  - Real test validator and WSOL operations
  - Campaign deployment with actual token transfers
  - Database vs blockchain consistency validation
  - Vault funding verification and balance checking

**🚧 In Progress:**

- **Deployment State Management**: Architecture improvements for campaign activation logic
- **API Server Infrastructure**: Proof serving for dApp integration
- **Enhanced Fixture Generator**: Campaign-organized directory structure with HD wallets

**📋 Next Priorities:**

1. **Robust State Management**: Implement proper deployment state validation
2. **Claiming Infrastructure**: API server and `claim-tokens` command
3. **Production Testing**: Cross-network and large-scale validation
4. **dApp Frontend**: User interface for claiming tokens

**🚨 Critical Achievements:**

- **Token Decimal Safety**: Fixed dangerous hardcoded decimal assumptions that could cause 1000x overfunding
- **Idempotent Deployment**: Safe re-runs prevent accidental double-funding
- **Comprehensive Testing**: Real blockchain testing with actual token operations
- **Secrets Management**: Production-ready team keypair management

## 6. Key Design Decisions & Implementation Notes

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

## 7. Benchmarking Plan (using Mollusk SVM)

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

## 8. Technical Debt & Code Quality Issues 🚨

### Database Connection Management

- **Issue**: Ad-hoc database connection handling across commands
- **Problems**:
  - Opening `Connection::open(db_path)` in multiple places within same function
  - Passing `PathBuf` instead of connections, leading to repeated opens
  - No encapsulation of common database operations
  - Inconsistent error handling for database operations
- **Solution Needed**:
  - Create `CampaignDatabase` struct to encapsulate all database operations
  - Centralized connection management and query methods
  - Consistent error handling and connection pooling if needed
- **Priority**: High - affects maintainability and performance

### RPC Client Management

- **Issue**: Scattered RPC client creation and configuration
- **Problems**:
  - Creating `RpcClient::new_with_commitment()` in every command
  - Duplicate RPC connection logic and error handling
  - No centralized configuration for timeouts, retry logic, etc.
  - Missing abstraction for common blockchain operations
- **Solution Needed**:
  - Create `prism-protocol-client` crate with unified client interface
  - Encapsulate common operations (get account, send transaction, etc.)
  - Centralized RPC configuration and error handling
  - Connection pooling and retry mechanisms
- **Priority**: Medium-High - improves reliability and reduces duplication

### Code Organization & Patterns

- **Issue**: Command modules doing too much, mixed concerns
- **Problems**:
  - Commands directly handling database operations instead of using services
  - Mixed business logic and I/O operations
  - Inconsistent patterns across similar commands
  - Copy-pasted code for common operations (reading campaign data, etc.)
- **Solution Needed**:
  - Extract business logic into service modules
  - Create shared utilities for common operations
  - Consistent error handling patterns
  - Better separation of concerns (CLI parsing vs business logic vs data access)
- **Priority**: Medium - technical debt that will compound over time

### Transaction Management & Observability

- **Issue**: Lack of transaction simulation and comprehensive logging
- **Problems**:
  - No pre-flight transaction simulation to catch errors early
  - Transaction signatures not consistently logged for explorer review
  - Difficult to debug failed transactions without signature tracking
  - No unified transaction building and submission pattern
- **Solution Needed**:
  - Implement `simulate_transaction` before all `send_transaction` calls
  - Centralized transaction logging with explorer URLs
  - Standardized transaction building pattern across all commands
  - Optional `--dry-run` mode for all commands that submit transactions
- **Priority**: High - essential for debugging and user experience

### CLI Architecture Consolidation

## 9. Documentation Checklist

- [x] `

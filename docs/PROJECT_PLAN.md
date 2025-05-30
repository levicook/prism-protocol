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

**✅ MAJOR MILESTONE: END-TO-END TOKEN CLAIMING OPERATIONAL!**

- **🎉 Complete Working System**: From fixture generation → compilation → deployment → **successful token claiming**
- **🔧 Critical Bug Fixed**: Vault address derivation now uses correct fingerprints
- **🛡️ Production-Ready Security**: Double-spend protection, proper claim validation, automatic token account creation
- **📊 Comprehensive Verification**: Database + blockchain consistency validation working
- **🧪 Real Blockchain Testing**: Full test validator integration with actual WSOL operations

**✅ Completed Infrastructure:**

- ✅ **Core On-Chain Program**: All essential instructions (campaign, cohort, vault, claiming)
- ✅ **Complete Crate Architecture**: Modular separation of concerns across 4 focused crates
- ✅ **Comprehensive Test Suite**: Unit, integration, and end-to-end testing with Mollusk SVM
- ✅ **CLI Phase 0-2 COMPLETE**: Fixture generation, campaign compilation, full deployment
- ✅ **CLI Phase 3 PARTIAL**: Query/eligibility infrastructure with database + blockchain verification
- ✅ **Secrets & Configuration Management**: Team-based encryption, organized CLI configs
- ✅ **Critical Safety Features**: Token decimal safety, idempotent deployment, comprehensive pre-flight checks

**🚨 CRITICAL FINDING: Technical Debt Blocking API Server**

**Analysis revealed extensive technical debt that MUST be addressed before API server implementation:**

- **Database Connection Chaos**: **19+ redundant `Connection::open()` calls** across CLI commands
- **RPC Client Duplication**: **6+ identical RPC client setups** with no pooling or error handling
- **Copy-Paste Architecture**: Every command reimplements database reading, pubkey parsing, error handling
- **Missing Abstractions**: Raw SPL token byte scanning, no transaction simulation, inconsistent logging

**📋 UPDATED NEXT PRIORITIES (CRITICAL ORDER):**

### **IMMEDIATE NEXT PRIORITIES** ⚡

**🎯 Phase 3A: Infrastructure Cleanup (PRIORITY 1 - BLOCKING API SERVER)**

- ✅ **CSV Schema Formalization** - **COMPLETED** ✨
  - ✅ Created dedicated `prism-protocol-csvs` crate
  - ✅ Authoritative schema definitions for `campaign.csv` and `cohorts.csv`
  - ✅ Cross-CSV validation (`validate_csv_consistency()`)
  - ✅ Type-safe serialization/deserialization with proper error handling
  - ✅ Comprehensive test coverage with version management
  - **Impact**: API server can now safely accept CSV uploads with guaranteed schema consistency

- ⏳ **Database Connection Management** (IN PROGRESS)
  - **Problem**: 19+ redundant `Connection::open()` calls across CLI commands
  - **Solution**: Create unified `CampaignDatabase` interface in `prism-protocol-db` crate
  - **Files to Refactor**: All command files with scattered database connections

- ⏳ **Campaign Compilation Consolidation** (PENDING)
  - **Problem**: Campaign compilation logic (CSV → merkle trees → SQLite) scattered across CLI commands
  - **Solution**: Move to `prism-protocol-db` crate as factory method: `CampaignDatabase::compile_from_csvs(campaign_rows, cohorts_rows, output_path)`
  - **Benefits**: Reusable compilation logic for CLI and future API endpoints

- ⏳ **CLI CSV Integration** (PENDING)  
  - **Problem**: CLI commands still use custom CSV parsing instead of `prism-protocol-csvs` crate
  - **Solution**: Convert `generate-fixtures`, `compile-campaign` to use `prism_protocol_csvs::{CampaignRow, CohortsRow}` types
  - **Files to Update**: `generate_fixtures.rs`, `compile_campaign.rs`

- ⏳ **Protocol Client Library** (PENDING)
  - **Problem**: Copy-paste RPC client code and byte scanning across commands  
  - **Solution**: Create `prism-protocol-client` crate with clean abstractions
  - **Include**: Token program types, account fetching, RPC connection management

### **NEXT: Phase 3B - API Server Implementation**

**Target: Week 2 of Next Sprint (2-3 days with clean foundation)**

1. **🌐 HTTP API Server** (`prism-protocol-cli serve-api`)

   - REST endpoints using shared database and client crates
   - Proof serving for frontend dApp integration
   - Campaign status and eligibility checking
   - Rate limiting, security, proper error handling

2. **🔗 Enhanced CLI Claim Integration**
   - `claim-tokens` command that uses API server for proof lookup
   - Simplified user experience with API-powered proof resolution

### **LATER: Phase 4+ - Production Features**

1. **🎨 dApp Frontend**: User interface for claiming tokens
2. **🏭 Production Readiness**: Cross-network testing, large-scale validation
3. **⚙️ Admin Operations**: Campaign management, pause/resume, token reclamation

**🎯 Success Metrics:**

- **Technical Debt Resolution**: Zero redundant database connections, unified RPC handling
- **API Server Performance**: <100ms response times, proper connection pooling
- **Developer Experience**: Clean, maintainable codebase ready for team scaling
- **User Experience**: Smooth claiming flow from API → dApp → successful token transfer

**📈 Impact of This Approach:**

- **Foundation First**: Clean architecture enables rapid feature development
- **Scalable Infrastructure**: Proper connection management handles production load
- **Maintainable Codebase**: Shared abstractions reduce copy-paste bugs
- **Team Velocity**: New developers can contribute without navigating technical debt

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

## 8. Critical Technical Debt & Code Quality Issues 🚨

### **✅ RESOLVED: CSV Schema Definition (COMPLETED)**

- **Previous Issue**: Loosely defined CSV interface between `generate-fixtures` and `compile-campaign`
- **Solution Implemented**: 
  - ✅ Created dedicated `prism-protocol-csvs` crate with authoritative schemas
  - ✅ Type-safe `CampaignCsvRow` and `CohortsCsvRow` definitions
  - ✅ Cross-file validation with `validate_csv_consistency()`
  - ✅ Version management and comprehensive test coverage
- **Result**: API server can now safely accept CSV uploads with guaranteed consistency

### **PRIORITY 1: Database Connection Management (BLOCKING API SERVER)**

- **Issue**: Extremely scattered database connection handling across ALL commands
- **Scale of Problem**:
  - `deploy_campaign.rs`: **9 separate `Connection::open()` calls**
  - `check_eligibility.rs`: **2 separate `Connection::open()` calls**
  - `claim_tokens.rs`: **3 separate `Connection::open()` calls**
  - `campaign_status.rs`: **3 separate `Connection::open()` calls**
  - `fund_vaults.rs`: **2 separate `Connection::open()` calls**
  - **Total: 19+ redundant database connections across codebase**
- **Specific Problems**:
  - Opening `Connection::open(db_path)` repeatedly within the SAME function
  - Passing `PathBuf` instead of connections, forcing repeated opens
  - No transaction management or connection pooling
  - Inconsistent error handling for database operations
  - **API server will amplify this problem 100x** with concurrent requests
- **Solution Required**:

  ```rust
  // Create new crate: `prism-protocol-db`
  pub struct CampaignDatabase {
      conn: Connection,
  }

  impl CampaignDatabase {
      pub fn open(path: &Path) -> Result<Self, DbError> { /* */ }
      pub fn read_campaign_info(&self) -> Result<CampaignInfo, DbError> { /* */ }
      pub fn read_cohort_data(&self) -> Result<Vec<CohortData>, DbError> { /* */ }
      pub fn read_claimant_eligibility(&self, pubkey: &Pubkey) -> Result<Vec<EligibilityInfo>, DbError> { /* */ }
      pub fn update_vault_funding(&mut self, /* ... */) -> Result<(), DbError> { /* */ }
      // ... all database operations
  }
  ```

- **Priority**: **CRITICAL** - Must complete before API server work

### **PRIORITY 2: RPC Client Management (BLOCKING API SERVER)**

- **Issue**: Duplicated RPC client creation and configuration across ALL commands
- **Scale of Problem**:
  - Every command creates `RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed())`
  - No centralized configuration, error handling, or retry logic
  - Missing abstraction for common blockchain operations
  - **API server will need shared RPC client pool** - current approach won't scale
- **Specific Problems**:
  - `deploy_campaign.rs`, `check_eligibility.rs`, `claim_tokens.rs`, `fund_vaults.rs`, `campaign_status.rs`, `query_claims.rs` all duplicate identical RPC setup
  - No connection pooling, timeouts, or retry mechanisms
  - Raw `get_account_data()` calls scattered everywhere
  - Missing transaction simulation and logging
- **Solution Required**:

  ```rust
  // Create new crate: `prism-protocol-client`
  pub struct PrismProtocolClient {
      rpc_client: RpcClient,
      program_id: Pubkey,
  }

  impl PrismProtocolClient {
      pub fn new(rpc_url: String) -> Result<Self, ClientError> { /* */ }
      pub fn get_campaign(&self, fingerprint: &[u8; 32], admin: &Pubkey) -> Result<Option<CampaignV0>, ClientError> { /* */ }
      pub fn get_cohort(&self, campaign: &Pubkey, merkle_root: &[u8; 32]) -> Result<Option<CohortV0>, ClientError> { /* */ }
      pub fn get_mint_info(&self, mint: &Pubkey) -> Result<MintInfo, ClientError> { /* */ }
      pub fn get_token_account_balance(&self, address: &Pubkey) -> Result<u64, ClientError> { /* */ }
      pub fn simulate_and_send_transaction(&self, tx: Transaction) -> Result<Signature, ClientError> { /* */ }
      // ... all blockchain operations
  }
  ```

- **Priority**: **CRITICAL** - Must complete before API server work

### **PRIORITY 3: Sketchy SPL Token Account Handling**

- **Issue**: Manual byte scanning and unsafe token account operations
- **Specific Problems Found**:
  - `deploy_campaign.rs:97-101`: Raw `get_account_data()` + `Mint::unpack()` for decimal fetching
  - Hardcoded WSOL address checking: `"So11111111111111111111111111111111111111112"`
  - No abstraction for common SPL token operations
  - Missing proper error handling for malformed token accounts
- **Solution Required**: Integrate into `PrismProtocolClient` with proper SPL token abstractions
- **Priority**: **HIGH** - Needed for API server token formatting

### **PRIORITY 4: Transaction Management & Observability**

- **Issue**: No transaction simulation, inconsistent logging, poor debugging experience
- **Specific Problems**:
  - No `simulate_transaction()` calls before `send_transaction()` - failures discovered too late
  - Transaction signatures scattered in println!() statements instead of structured logging
  - No explorer URL generation for easy debugging
  - Missing `--dry-run` capabilities across commands
  - No standardized transaction building patterns
- **Solution Required**:
  ```rust
  impl PrismProtocolClient {
      pub fn simulate_and_send_transaction(&self, tx: Transaction, dry_run: bool) -> Result<TransactionResult, ClientError> {
          if dry_run {
              let sim_result = self.rpc_client.simulate_transaction(&tx)?;
              return Ok(TransactionResult::Simulated(sim_result));
          }

          // Always simulate first in live mode
          let sim_result = self.rpc_client.simulate_transaction(&tx)?;
          if sim_result.value.err.is_some() {
              return Err(ClientError::SimulationFailed(sim_result));
          }

          let signature = self.rpc_client.send_transaction(&tx)?;
          println!("✅ Transaction: https://explorer.solana.com/tx/{}", signature);
          Ok(TransactionResult::Executed(signature))
      }
  }
  ```
- **Priority**: **HIGH** - Essential for API server reliability

### **PRIORITY 5: CLI Architecture Consolidation**

- **Issue**: Commands doing too much, mixed concerns, copied code patterns
- **Specific Problems**:
  - Every command implements its own database reading logic
  - Business logic mixed with I/O and CLI parsing
  - Copy-pasted error handling and validation patterns
  - No shared utilities for common operations
- **Examples of Duplication**:
  - Reading campaign info: `deploy_campaign.rs:383`, `check_eligibility.rs:183`, `campaign_status.rs:64`
  - Reading cohort data: `deploy_campaign.rs:422`, `fund_vaults.rs:374`
  - Pubkey parsing: `check_eligibility.rs:38-50`, `query_claims.rs:30-42`
- **Solution Required**: Extract business logic into service modules, create shared utilities
- **Priority**: **MEDIUM** - Technical debt that compounds over time

### **NEW CRITICAL ISSUE: Error Handling Inconsistency**

- **Issue**: Inconsistent error handling patterns across commands
- **Problems**:
  - Mix of `CliError::InvalidConfig()` and direct `map_err()` calls
  - Some errors use formatted strings, others use direct error propagation
  - Database errors sometimes wrapped, sometimes not
  - RPC errors handled differently across commands
- **Solution Required**: Standardize error handling patterns, better error context
- **Priority**: **MEDIUM-HIGH** - Will cause debugging issues in production

## **Updated Implementation Plan for API Server Success**

### **Phase 3A: Infrastructure Cleanup (MUST COMPLETE FIRST)**

**Target: Week 1 of API Server Sprint**

1. **🏗️ Create `prism-protocol-db` crate**

   - Encapsulate ALL database operations
   - Connection management and transaction support
   - Consistent error handling
   - **Replace all 19+ `Connection::open()` calls**

2. **🌐 Create `prism-protocol-client` crate**

   - Unified RPC client with connection pooling
   - Common blockchain operations (accounts, transactions, SPL tokens)
   - Transaction simulation and logging
   - **Replace all 6+ duplicated RPC client creations**

3. **🔧 Refactor CLI commands to use new crates**
   - Remove all direct database and RPC code
   - Standardize error handling patterns
   - Add `--dry-run` support across all commands

### **Phase 3B: API Server Implementation**

**Target: Week 2 of API Server Sprint**

1. **🌐 HTTP API Server** (`prism-protocol-cli serve-api`)

   - REST endpoints using shared database and client crates
   - Connection pooling for both database and RPC
   - Proper error handling and logging
   - Rate limiting and security

2. **🔗 Enhanced CLI Claim Integration**
   - `claim-tokens` command that uses API server for proof lookup
   - Use shared client for transaction handling

**Estimated Effort**:

- Phase 3A (Infrastructure): **3-4 days** (critical foundation)
- Phase 3B (API Server): **2-3 days** (straightforward with good foundation)

**Why This Order Matters**:

- The current codebase has **19+ database connections** and **6+ RPC clients** scattered everywhere
- API server with concurrent requests would amplify these problems exponentially
- Clean infrastructure makes API server implementation trivial
- Without cleanup first, API server will inherit all current technical debt and be fragile

## 9. Documentation Checklist

- [x] `

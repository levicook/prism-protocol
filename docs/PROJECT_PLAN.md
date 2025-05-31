# Prism Protocol: Project Plan & Checklist

## 1. Core Project Goal

To enable efficient, scalable, and verifiable token distribution on Solana, minimizing write contention and optimizing on-chain resources. (See `README.md` for full details).

## 🎯 **STRATEGIC PRIORITIES & USER EXPERIENCE ARCHITECTURE**

### **Current Status: Strong Foundations, Need User-Focused Strategy**

After one week of development, we've built incredibly solid technical foundations:
- ✅ **Financial Safety**: Precise decimal math eliminates funding errors
- ✅ **Infrastructure**: Clean abstractions, comprehensive testing, production-ready CLI
- ✅ **Protocol Design**: Campaign fingerprints, merkle proofs, vault distribution

**Critical Gap**: We've optimized for developers, not end users. Need clear user experience strategy.

### **User Experience Priority Ranking**

**Primary Users (95% of usage):**
1. **dApp Users** - Campaign deployers uploading CSVs, claimants connecting wallets
2. **CLI Users** - Technical campaign deployers comfortable with command-line tools

**Secondary Users (5% of usage):**
3. **SDK Integrators** - Developers building custom dApps/services
4. **API Consumers** - Programmatic access for data/automation

### **v0 vs v1 Performance Strategy**

**Current v0 (Proof of Concept):**
- ✅ Full merkle proofs (~32 bytes × tree depth)
- ✅ Individual transactions per operation
- ⚠️ **Performance claims unvalidated** - need concrete measurements

**Planned v1 (Production Optimization):**
- 🎯 **Trunk optimization**: Store intermediate tree levels on cohorts → shorter proofs
- 🎯 **CU optimization**: Measured compute unit consumption with accurate cost estimation
- 🎯 **Batch operations**: Transaction packing for deployment efficiency
- 🎯 **Priority fee strategy**: Optimal fee calculation for reliable execution

### **Performance Validation Requirements**

**Critical Measurements Needed:**
- **CU Consumption**: Exact compute units per instruction (claim, deploy, etc.)
- **Transaction Sizes**: Bytes consumed under various scenarios (1K vs 100K claimants)
- **Write Contention**: Actual concurrent claim performance testing
- **Proof Sizes**: Current vs optimized merkle proof bandwidth requirements
- **On-chain Storage**: Account rent costs at scale (campaigns, cohorts, receipts)

**Validation Methodology:**
- Mollusk SVM benchmarking with realistic datasets
- Devnet stress testing with concurrent users
- Cost modeling for different campaign sizes
- Comparison with naive airdrop approaches

### **User Experience Architecture (Critical Path)**

**Campaign Deployer Journey:**
```
CSV Upload → dApp Validation → Budget Calculation → Deploy → Monitor
     ↓              ↓               ↓              ↓         ↓
   (or CLI)    Fee Estimation   Vault Funding    Status   Claims
```

**Claimant Journey:**
```
Connect Wallet → Check Eligibility → Claim Tokens → View History
      ↓               ↓                  ↓             ↓
   (instant)    (database query)   (merkle proof)  (receipts)
```

**Missing Components:**
- 🚨 **dApp Frontend**: Primary user interface (most critical gap)
- 🚨 **CSV Upload Interface**: Campaign creation without CLI knowledge
- 🚨 **Claim Status Dashboard**: User-friendly claim tracking
- ⚠️ **dApp Template**: Next.js template for forks (mono-repo issue)

### **Immediate Strategic Priorities**

1. **Performance Validation Sprint** (1 week)
   - Measure actual CU consumption, transaction sizes, write contention
   - Validate or refine performance claims with concrete data
   - Document optimization opportunities for v1

2. **User Experience Definition** (1 week)
   - Design primary user journeys (campaign deployer + claimant)
   - Define dApp requirements and architecture
   - Resolve mono-repo vs template strategy

3. **v1 Optimization Planning** (ongoing)
   - Trunk optimization specification for shorter proofs
   - CU optimization roadmap with priority fee strategy
   - Batch operation enhancement beyond current transaction packing

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

- **✅ COMPLETED: Infrastructure Foundation (MERGED AND VALIDATED!)**

  1. **CSV Schema Formalization** ✅ COMPLETED ✨

     - ✅ Created `prism-protocol-csvs` crate with authoritative schemas
     - ✅ Cross-CSV validation (`validate_csv_consistency()`)
     - ✅ Type-safe serialization/deserialization with proper error handling
     - ✅ 5/5 tests passing with version management
     - **Impact**: API server foundation ready for CSV uploads

  2. **Database Interface Implementation** ✅ COMPLETED ✨

     - ✅ Complete `prism-protocol-db` crate with `CampaignDatabase` interface
     - ✅ Schema management, connection handling, all CRUD operations
     - ✅ 5/5 tests passing including error handling
     - **Impact**: Eliminated scattered `Connection::open()` calls - ready for API server

  3. **Client Infrastructure** ✅ COMPLETED ✨
     - ✅ Complete `prism-protocol-client` crate with `PrismProtocolClient`
     - ✅ Unified RPC operations, SPL token management, transaction simulation
     - ✅ Clean abstractions for all protocol operations
     - **Impact**: Ready to eliminate scattered RPC client creation

- **✅ COMPLETED: CLI Modernization (Phase 3B)**

  **Status**: ✅ ALL CLI COMMANDS MODERNIZED - Zero scattered RPC client calls remaining

  **✅ Modernized Commands:**

  - ✅ `check_eligibility.rs` - Using `CampaignDatabase` + `PrismProtocolClient`
  - ✅ `deploy_campaign.rs` - Using `CampaignDatabase` + `PrismProtocolClient`
  - ✅ `campaign_status.rs` - Using `CampaignDatabase` + `PrismProtocolClient`
  - ✅ `query_claims.rs` - Using `CampaignDatabase` + `PrismProtocolClient`
  - ✅ `claim_tokens.rs` - Using `CampaignDatabase` + `PrismProtocolClient`

  **🎉 Technical Debt Elimination Results:**

  - ✅ **Zero `RpcClient::new_with_commitment()` calls** in CLI commands
  - ✅ **Zero scattered database connections** - all using `CampaignDatabase`
  - ✅ **All commands using unified `PrismProtocolClient`** for blockchain operations
  - ✅ **Consistent error handling patterns** across all commands
  - ✅ **25/25 tests passing** after modernization

  **Migration Pattern Used:**

  ```rust
  // BEFORE (legacy):
  let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

  // AFTER (modernized):
  let client = PrismProtocolClient::new(rpc_url)
      .map_err(|e| CliError::InvalidConfig(format!("Failed to create RPC client: {}", e)))?;
  ```

  **Validated Infrastructure**: All commands now use proper abstractions, ready for API server

- **📋 PLANNED: API Server Implementation (Phase 3C)**

  **Status**: NOT STARTED - infrastructure foundation now ready

  **Key Components:**

  - ❌ HTTP API Server (`prism-protocol-cli serve-api`) - Core proof serving
  - ❌ REST endpoints for eligibility and proof lookup
  - ❌ Enhanced CLI claim integration using API server

  **Architecture Ready:**

  - ✅ Database pooling via `CampaignDatabase`
  - ✅ RPC operations via `PrismProtocolClient`
  - ✅ CSV schema validation via `prism-protocol-csvs`

**Phase 4: Campaign Management & Production Readiness 📋 PLANNED**

- **Purpose:** Administrative operations, production deployment, and campaign creation tools

- **🚀 CRITICAL ARCHITECTURAL IMPROVEMENTS NEEDED**

  **Transaction Architecture Overhaul**

  _Current Problem_: Deploy command creates 50+ individual transactions with scattered transaction building/sending logic:

  - 1 campaign initialization
  - 5+ cohort initializations
  - 20+ vault creations
  - 20+ vault funding operations
  - 1 campaign activation
  - **Total: ~60 seconds deployment time, ~$1.25 in fees**

  _Required Components_:

  1. **Deploy Planner Abstraction** 📋 NEW

     - **Purpose**: Determine what instructions need to be built based on DB signatures and on-chain state
     - **Inputs**: `CampaignDatabase`, `PrismProtocolClient`, campaign fingerprint
     - **Logic**:
       - Check DB for existing deployment signatures
       - Cross-reference with actual on-chain account existence (surgical `get_account` calls)
       - Generate instruction plan for missing/incomplete deployments
       - Support for partial deployment recovery (handle failed mid-deployment scenarios)
     - **Outputs**: `Vec<DeploymentStep>` with instruction builders and dependencies
     - **Benefits**: Idempotent deployments, clear deployment state visibility

  2. **Generic Instruction/Transaction Packer** 📋 CONFIRMED IN PLAN

     - **Purpose**: Pack instructions into optimally-sized transactions
     - **Inputs**: `Vec<Instruction>`, `&Keypair` (admin signer), transaction size limits
     - **Logic**:
       - Respect instruction dependencies and execution order
       - Pack by transaction size limits (not arbitrary batching)
       - Handle cross-instruction account dependencies
       - Generate multiple transactions when size limits exceeded
     - **Outputs**: `Vec<Transaction>` ready for transmission
     - **Generic Usage**: Useful for deploy, claim batching, any multi-instruction operations

  3. **Unified Transaction Retry Utility** 📋 NEW

     - **Purpose**: Single retry mechanism with proper re-signing
     - **Inputs**: `Vec<Transaction>`, `&Keypair`, `&RpcClient`, retry config
     - **Logic**:
       - Fresh blockhash fetching for each retry attempt
       - Re-signing with admin keypair (critical for retry success)
       - Exponential backoff with jitter
       - Transaction confirmation waiting
       - Detailed error logging with explorer URLs
     - **Benefits**: Consistent retry behavior, proper error handling

  4. **Database Deployment Coordinator** 📋 NEW
     - **Purpose**: Track deployment signatures and state in DB
     - **Operations**:
       - `mark_campaign_deployed(signature)`
       - `mark_cohort_deployed(cohort_name, signature)`
       - `mark_vault_created(cohort, vault_index, signature)`
       - `mark_vault_funded(cohort, vault_index, signature, amount)`
       - `mark_campaign_activated(signature)`
     - **State Queries**: `get_deployment_status()` for resume/retry scenarios
     - **Benefits**: Full deployment auditability, recovery from partial failures

  _Expected Impact_:

  - **50+ transactions → 3-5 transactions** (intelligent batching)
  - **~60 seconds → ~10 seconds** deployment time
  - **~$1.25 → ~$0.15** in transaction fees
  - **Robust failure recovery** with clear deployment state tracking

- **🔧 CRITICAL CLIENT IMPROVEMENTS (Anemic Client Issues)**

  **Current Problem**: PrismProtocolClient is anemic - CLI commands resort to raw RPC client for many operations

  _Specific Raw RPC Usage Identified_:

  - **Blockhash Operations**: `rpc_client.get_latest_blockhash()` (used 8+ times in deploy_campaign.rs)
  - **Transaction Sending**: `rpc_client.send_and_confirm_transaction_with_spinner_and_config()` (used 6+ times)
  - **Account Existence Checks**: `rpc_client.get_account(vault_address).is_ok()` for vault detection
  - **Balance Queries**: `rpc_client.get_balance(&admin_pubkey)` for SOL balance checking
  - **Connection Testing**: `rpc_client.get_slot()` for RPC stability verification
  - **Admin Token Balance**: Manual ATA address derivation + raw token account fetching

  _Required Client Enhancements_:

  ```rust
  impl PrismProtocolClient {
      // Transaction Management
      pub fn build_and_send_transaction(&self, instructions: Vec<Instruction>, signers: &[&dyn Signer]) -> ClientResult<Signature>
      pub fn simulate_transaction(&self, instructions: &[Instruction]) -> ClientResult<SimulationResult>
      pub fn send_transaction_with_retry(&self, tx: Transaction, max_retries: u8) -> ClientResult<Signature>

      // Account Existence & State Checking
      pub fn account_exists(&self, address: &Pubkey) -> ClientResult<bool>
      pub fn get_sol_balance(&self, address: &Pubkey) -> ClientResult<u64>
      pub fn get_token_balance(&self, owner: &Pubkey, mint: &Pubkey) -> ClientResult<u64>

      // Campaign State Verification
      pub fn verify_campaign_ready_for_activation(&self, fingerprint: &[u8; 32]) -> ClientResult<ActivationStatus>
      pub fn get_campaign_deployment_status(&self, fingerprint: &[u8; 32]) -> ClientResult<DeploymentStatus>

      // Batch Operations
      pub fn get_multiple_accounts(&self, addresses: &[Pubkey]) -> ClientResult<Vec<Option<Account>>>
      pub fn check_vault_funding_status(&self, cohort: &Pubkey, vault_count: u8) -> ClientResult<Vec<VaultStatus>>
  }
  ```

  _Benefits_:

  - **Eliminate all raw RPC usage** in CLI commands
  - **Consistent error handling** across all blockchain operations
  - **Built-in retry logic** with proper re-signing
  - **Campaign-aware operations** that understand protocol semantics
  - **Batch optimizations** for multi-account queries

- **🔄 CAMPAIGN BUDGET & SAFE MATH STATUS** 📋 IN PROGRESS

  **Current Implementation Status**:

  - ✅ **Decimal Math Foundation**: `rust_decimal::Decimal` integrated in fixture generator
  - ✅ **Precise CSV Generation**: Percentages sum to exactly 100.0% in cohorts.csv
  - ✅ **Budget Parsing**: CLI accepts human-readable token amounts (e.g., "1000.5")
  - ✅ **Base Unit Conversion**: Proper multiplication by 10^decimals for mint operations
  - ⚠️ **Partial Integration**: Only fixture generator uses `Decimal`, compile_campaign still uses approximations

  **Remaining Work Required**:

  1. **Complete `compile_campaign.rs` Integration** 📋 HIGH PRIORITY

     ```rust
     // CURRENT (problematic):
     let budget_f64: f64 = budget.parse()?;
     let budget_base_units = (budget_f64 * (10_u64.pow(mint_decimals as u32) as f64)) as u64;

     // REQUIRED (precise):
     let budget_decimal = Decimal::from_str(&budget)?;
     let base_unit_multiplier = Decimal::from(10_u64.pow(mint_decimals as u32));
     let budget_base_units = budget_decimal.checked_mul(base_unit_multiplier)?.to_u64()?;
     ```

  2. **Database Schema Updates** 📋 MEDIUM PRIORITY

     - Store budget amounts as precise decimal strings (not floating point)
     - Add mint decimals to campaign_info table for validation
     - Update vault requirements calculation to use precise math

  3. **Validation Layer** 📋 MEDIUM PRIORITY
     - Verify budget × cohort percentages = expected total allocations
     - Cross-check mint decimals between CLI input and blockchain state
     - Add overflow protection for large token amounts

  **Financial Safety Impact**:

  - **Current Risk**: Floating point errors can cause 0.1% funding discrepancies
  - **With Complete Integration**: Mathematically precise token allocations
  - **Example**: 1M USDC campaign difference: $1,000 error vs. $0 error

- **🎯 IMMEDIATE NEXT PRIORITIES**

  1. **Complete Campaign Budget Integration** (1-2 days)

     - Fix `compile_campaign.rs` to use `Decimal` throughout
     - Update database schema for precise storage
     - Add validation layer for budget consistency

  2. **Transaction Architecture Implementation** (3-4 days)

     - Deploy planner abstraction
     - Generic instruction/transaction packer
     - Unified retry utility
     - Database deployment coordinator

  3. **Enhanced PrismProtocolClient** (2-3 days)
     - Add all missing abstractions to eliminate raw RPC usage
     - Campaign-aware operations and state checking
     - Batch optimization capabilities

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

**✅ MAJOR STATUS UPDATE: Infrastructure Foundation Complete**

**🎉 CRITICAL ACHIEVEMENT: All Infrastructure Crates Operational**

- **25/25 tests passing** across all workspace crates
- **Zero compilation errors** - clean, working foundation
- **Proven CLI integration** - `check_eligibility` demonstrates full infrastructure usage
- **Ready for API server** - all abstractions in place

**📊 Technical Debt Elimination Status:**

| Issue                | Before                                   | After                                     | Status        |
| -------------------- | ---------------------------------------- | ----------------------------------------- | ------------- |
| Database Connections | 19+ scattered `Connection::open()` calls | Single `CampaignDatabase` interface       | ✅ ELIMINATED |
| RPC Client Creation  | 6+ scattered `RpcClient::new()` calls    | Single `PrismProtocolClient` interface    | ✅ ELIMINATED |
| CSV Schema Chaos     | Loosely defined interface                | Authoritative `prism-protocol-csvs` crate | ✅ ELIMINATED |
| SPL Token Handling   | Raw byte scanning, unsafe operations     | Clean `anchor_spl` abstractions           | ✅ ELIMINATED |

**🎯 IMMEDIATE NEXT PRIORITIES (1-2 days each):**

### **✅ COMPLETED: CLI Modernization + Query Claims Implementation**

- ✅ **Target**: Migrate remaining 3 commands to use `PrismProtocolClient`
- ✅ **Effort**: Completed in ~2 hours total (pattern was proven effective)
- ✅ **Validation**: Zero scattered RPC client calls in entire codebase achieved
- ✅ **BONUS**: Implemented fully functional `query_claims` command using existing infrastructure
  - ✅ Uses `CampaignDatabase` + `PrismProtocolClient` for clean architecture
  - ✅ Automatically detects keypair files vs. pubkey strings
  - ✅ Queries all cohorts for claim receipts using `get_claim_receipt_v0()`
  - ✅ Beautiful output with timestamps, vault assignments, and explorer links
  - ✅ **Simple approach**: No complex `getProgramAccounts` filtering needed

### **🎯 CURRENT PRIORITY: API Server Implementation**

- **Target**: HTTP REST API using completed infrastructure (`serve-api` command)
- **Effort**: 2-3 days (foundation makes this straightforward)
- **Impact**: Complete claiming ecosystem with dApp integration ready
- **Architecture Ready**: All infrastructure crates (`prism-protocol-db`, `prism-protocol-client`, `prism-protocol-csvs`) operational with 25/25 tests passing

**🚀 PROJECT MOMENTUM: Foundation → Implementation**

The hard architectural work is **COMPLETE**. Next phase is rapid implementation using proven patterns.

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

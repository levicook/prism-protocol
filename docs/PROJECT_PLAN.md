# Prism Protocol: Project Plan v2

**Vision**: Trust-minimized token distribution with cryptographic verification at every step.

**Strategic Foundation**: Based on validated [DEPLOYMENT_ARCHITECTURE.md](./DEPLOYMENT_ARCHITECTURE.md) - IPFS-first, registration-tracked, cryptographically verified deployment pipeline.

## 🎯 Executive Summary

### **What We Have Built (Current v0 System)**

**✅ Solid Foundation:**
- Complete CLI with 10 functional commands
- Infrastructure crates: `prism-protocol-db`, `prism-protocol-client`, `prism-protocol-csvs`
- End-to-end claiming works (CSV → compile → deploy → claim)
- Precise decimal math throughout
- Comprehensive test coverage

**⚠️ Current Limitations:**
- **Direct deployment pattern** (missing IPFS + activation controls)
- **No trust verification** (can't independently verify deployments)
- **Mixed abstractions** (commands use both new infrastructure and raw RPC)
- **No automated site generation** (manual claim process only)

### **Strategic Target (Validated v1 Architecture)**

**🎯 Trust-Minimized Deployment Pipeline:**

```
CSV Files (Immutable) →
  IPFS Publishing (Content-Addressed) →
    Campaign Compilation (Deterministic) →
      On-Chain Deployment (Registration Pattern) →
        Activation Controls (Comprehensive Validation) →
          Automated Site Generation (Zero-Touch)
```

**🔑 Key Benefits:**
- **Campaign creators**: Can verify deployed data matches their CSV
- **Token claimants**: Can independently verify eligibility amounts  
- **Auditors**: Can reproduce entire campaign locally from CSV files
- **Platform operators**: Zero-trust architecture with cryptographic verification

## 🚨 IMMEDIATE PRIORITY: Verification Workflows

**Issue Identified**: Current verification workflows in PROTOCOL_ARCHITECTURE.md need design work to properly reflect IPFS-first architecture.

**Current Problems:**
- Verification commands still reference local CSV files instead of IPFS sources
- Mixed patterns between IPFS-hash-based verification and file-based verification
- Self-hosted integration patterns are placeholders, not ready for implementation
- Command-line examples don't consistently show the IPFS-first verification flow

**Required Design Work:**

### **1. IPFS-First Verification Commands (PRIORITY 1)**

**Current (inconsistent):**
```bash
# Some commands use local files
prism-protocol-cli verify-campaign \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv \
  --campaign-db-in campaign.db

# Others use IPFS hashes  
prism-protocol-cli verify-campaign \
  --campaign-ipfs-hash QmCampaignDB789... \
  --check-all-targets
```

**Need to Design:** Consistent IPFS-first verification workflow that:
- Fetches original CSVs from IPFS using hashes stored in campaign.db
- Supports both "I have the IPFS hash" and "I have local files" entry points
- Clearly shows the verification chain: IPFS → CSV → Database → On-chain
- Provides useful output for each verification step

### **2. Self-Hosted Integration Workflow (PRIORITY 2)**

**Current**: Marked as placeholder in PROTOCOL_ARCHITECTURE.md
**Need to Design:**
- How do users discover campaign IPFS hashes for self-hosting?
- What does the site generation command actually look like?
- How do generated sites load data from IPFS vs embedded data?
- What templates exist and how do they work?

### **3. Platform Integration Verification (PRIORITY 3)**

**Need to Design:**
- How do hosted platforms prove they're serving correct data?
- What does cross-platform verification look like?
- How do users verify a hosted site against IPFS sources?

**Action Items:**
- [ ] Design complete IPFS-first verification command interface
- [ ] Create verification workflow diagrams showing all entry points
- [ ] Prototype verification commands to validate user experience
- [ ] Update PROTOCOL_ARCHITECTURE.md with finalized verification patterns
- [ ] Design self-hosted integration workflow with concrete commands

## 📊 Current System Analysis

### **CLI Commands (All Functional)**

| Command | Status | Uses New Infrastructure | Raw RPC Usage |
|---------|--------|------------------------|---------------|
| `generate-fixtures` | ✅ Working | Partial | Yes |
| `compile-campaign` | ✅ Working | Yes | Minimal |
| `deploy-campaign` | ✅ Working | Partial | Yes |
| `campaign-status` | ✅ Working | Partial | Yes |
| `claim-tokens` | ✅ Working | Partial | Yes |
| `check-eligibility` | ✅ Working | Yes | Minimal |
| `query-claims` | ✅ Working | Yes | Minimal |
| `pause-campaign` | ✅ Working | Partial | Yes |
| `resume-campaign` | ✅ Working | Partial | Yes |
| `reclaim-tokens` | ✅ Working | Partial | Yes |

**Summary**: Hybrid state - all commands work, infrastructure exists, but modernization incomplete.

### **Infrastructure Assessment**

**✅ Completed Infrastructure:**
- `prism-protocol-db`: Unified database interface (eliminates scattered connections)
- `prism-protocol-client`: RPC abstraction layer (reduces but doesn't eliminate raw usage)
- `prism-protocol-csvs`: Authoritative CSV schemas with `Decimal` precision
- `prism-protocol-sdk`: Address finders and instruction builders
- `prism-protocol-merkle`: Off-chain tree construction and proof generation

**⚠️ Architecture Gaps vs DEPLOYMENT_ARCHITECTURE.md:**
- **No IPFS integration** in deploy command
- **No activation controls** (campaigns activate immediately on deploy)
- **No registration pattern** (direct deployment without readiness validation)
- **No automated site generation** (no event monitoring or IPFS fetching)

## 🚀 Strategic Implementation Plan

### **Phase 1: Core v1 Architecture (2-3 weeks)**

**Goal**: Implement the validated DEPLOYMENT_ARCHITECTURE.md patterns

#### **1.1 Enhanced Deploy Command (Week 1)**

**Current**: Direct deployment (v0 pattern)
```bash
prism-protocol-cli deploy-campaign --campaign-db-in campaign.db --admin-keypair admin.json
# → Deploys directly to Solana, activates immediately
```

**Target**: IPFS-first atomic deployment (v1 pattern)
```bash
prism-protocol-cli deploy-campaign --campaign-db-in campaign.db --admin-keypair admin.json --go-live-date "2024-03-15T10:00:00Z"
# → 1. Publish CSVs to IPFS, 2. Deploy to Solana with IPFS hashes, 3. Remains INACTIVE
```

**Implementation Tasks:**
- [ ] Add IPFS client integration (`kubo` or `ipfs-http-client`)
- [ ] Enhance on-chain `CampaignV0` with IPFS hash fields and activation controls
- [ ] Modify deploy command to publish CSVs → deploy with hashes → update campaign.db
- [ ] Add go-live date parameter and embed in on-chain campaign
- [ ] Keep campaigns inactive by default after deployment

#### **1.2 Registration Pattern Implementation (Week 1-2)**

**Current**: Direct vault creation and funding
**Target**: Init/activate pattern with registration arrays

**New On-Chain Instructions:**
```rust
pub fn init_campaign_v1(ctx: Context<InitCampaignV1>, expected_cohort_count: u8, go_live_timestamp: i64) -> Result<()>
pub fn init_cohort_v1(ctx: Context<InitCohortV1>, expected_vault_count: u8) -> Result<()>
pub fn activate_vault_v1(ctx: Context<ActivateVaultV1>, vault_index: u8) -> Result<()>
pub fn activate_cohort_v1(ctx: Context<ActivateCohortV1>, cohort_index: u8) -> Result<()>
pub fn activate_campaign_v1(ctx: Context<ActivateCampaignV1>, final_db_ipfs_hash: [u8; 32]) -> Result<()>
```

**Enhanced State Structures:**
```rust
#[account]
pub struct CampaignV1 {
    // Existing fields
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub fingerprint: [u8; 32],
    
    // IPFS integration
    pub campaign_csv_ipfs_hash: [u8; 32],
    pub cohorts_csv_ipfs_hash: [u8; 32],  
    pub campaign_db_ipfs_hash: [u8; 32],  // Set during activation
    
    // Registration pattern
    #[max_len(MAX_COHORTS_PER_CAMPAIGN)]
    pub cohorts: Vec<Pubkey>,  // Cohorts register here
    
    // Activation controls
    pub is_active: bool,
    pub go_live_timestamp: i64,
    pub bump: u8,
}
```

#### **1.3 Activation Command (Week 2)**

**New Command**: Campaign activation with comprehensive validation
```bash
prism-protocol-cli activate-campaign --campaign-db-in campaign.db --admin-keypair admin.json
# → 1. Validate all cohorts/vaults ready, 2. Publish final DB to IPFS, 3. Activate campaign
```

**Activation Requirements:**
- All cohorts deployed and registered with campaign
- All vaults funded with correct token amounts
- Final campaign.db published to IPFS
- Admin signature required
- Campaign not already active

#### **1.4 Enhanced Verification (Week 2-3)**

**Current**: Basic on-chain status checking
**Target**: Cryptographic verification chain

```bash
prism-protocol-cli verify-campaign \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv \
  --campaign-db-in campaign.db \
  --check-ipfs-integrity
```

**Verification Steps:**
1. Recompile CSVs → verify campaign.db fingerprint matches
2. Check IPFS hashes in campaign.db → verify CSVs published correctly
3. Check on-chain merkle roots → verify match campaign.db
4. Validate activation status and funding completeness

### **Phase 2: Automation Infrastructure (Week 3-4)**

**Goal**: Enable zero-touch site generation from campaign activations

#### **2.1 Event Monitoring (Week 3)**

**New Command**: Monitor campaign activations
```bash
prism-protocol-cli monitor-campaigns \
  --program-id <PRISM_PROGRAM_ID> \
  --auto-generate-sites \
  --output-dir ./generated-sites/
```

**Architecture:**
- Monitor `CampaignActivated` events containing IPFS hashes
- Extract campaign data from IPFS using content hashes
- Generate static claim sites with embedded campaign data
- Deploy to CDN or static hosting automatically

#### **2.2 Static Site Generation (Week 4)**

**New Command**: Generate claim sites from campaign data
```bash
prism-protocol-cli generate-claim-site \
  --campaign-db-in campaign.db \
  --template minimal \
  --output ./claim-site
```

**Templates:**
- **Minimal**: Basic claim interface with wallet connection
- **Modern**: Styled UI with progress indicators and transaction status
- **Corporate**: Customizable branding and company integration

### **Phase 3: Client SDK (Week 4-5)**

**Goal**: Enable dApp developers to integrate claiming functionality

#### **3.1 Basic Client SDK**

**Package**: `@prism-protocol/client-sdk` (TypeScript/JavaScript)

**Core Functionality:**
```typescript
// Wallet connection and eligibility checking
const eligibility = await prismClient.checkEligibility(walletAddress, campaignDb);

// Transaction building for claims
const claimTx = await prismClient.buildClaimTransaction(walletAddress, cohortProof);

// Bundle size: <50KB gzipped (essential for web usage)
```

**Key Features:**
- Merkle proof validation (client-side verification)
- Transaction building for claim instructions
- Wallet adapter integration (Phantom, Solflare, etc.)
- TypeScript definitions for type safety

## 🔧 Technical Implementation Details

### **IPFS Integration Strategy**

**Approach**: Use `kubo` (IPFS HTTP API) for reliable, production-ready IPFS operations

**Content Publishing Pattern:**
```rust
// Early publishing (safe, immutable inputs)
let campaign_csv_hash = ipfs_client.publish_file("customers.csv").await?;
let cohorts_csv_hash = ipfs_client.publish_file("cohorts.csv").await?;

// Store hashes in campaign.db immediately
db.store_ipfs_hash("campaign_csv_ipfs_hash", &campaign_csv_hash)?;
db.store_ipfs_hash("cohorts_csv_ipfs_hash", &cohorts_csv_hash)?;

// Late publishing (when deployment complete)
let final_db_hash = ipfs_client.publish_file("campaign.db").await?;
// Provided during activate_campaign instruction
```

### **Database Schema Enhancements**

**New IPFS Tracking Table:**
```sql
CREATE TABLE ipfs_hashes (
    key TEXT PRIMARY KEY,
    hash TEXT NOT NULL,
    published_at INTEGER NOT NULL
);

-- Example entries:
-- ('campaign_csv_ipfs_hash', 'QmCustomersHash123...', timestamp)
-- ('cohorts_csv_ipfs_hash', 'QmCohortsHash456...', timestamp)  
-- ('campaign_db_ipfs_hash', 'QmFinalDBHash789...', timestamp)
```

**Enhanced Campaign Info:**
```sql
ALTER TABLE campaign ADD COLUMN go_live_timestamp INTEGER;
ALTER TABLE campaign ADD COLUMN is_active BOOLEAN DEFAULT FALSE;
```

### **Error Handling & Recovery**

**Deployment Failure Recovery:**
- IPFS publishing failures → retry with exponential backoff
- Partial on-chain deployment → resume from last successful step
- Activation validation failures → clear error messages with specific remediation

**Verification Failures:**
- IPFS content mismatch → detailed diff output showing discrepancies
- On-chain state inconsistency → specific account addresses and expected vs actual values
- CSV recompilation mismatch → highlight which merkle roots don't match

## 📋 Success Metrics

### **Phase 1 Success Criteria**

**Functional:**
- [ ] Deploy command publishes CSVs to IPFS before on-chain deployment
- [ ] Campaigns remain inactive until explicitly activated
- [ ] Activation command validates all components ready before enabling claims
- [ ] Verify command cryptographically validates entire deployment chain

**Performance:**
- [ ] IPFS publishing completes in <30 seconds for typical campaigns (10K claimants)
- [ ] Activation validation runs in <10 seconds
- [ ] Verification against CSVs completes in <5 seconds

### **Phase 2 Success Criteria**

**Automation:**
- [ ] Event monitoring detects campaign activations within 30 seconds
- [ ] Static site generation completes in <2 minutes
- [ ] Generated sites load in <3 seconds globally

### **Phase 3 Success Criteria**

**Developer Experience:**
- [ ] Client SDK bundle size <50KB gzipped
- [ ] TypeScript definitions provide complete type safety
- [ ] Integration examples work with major wallet providers

## 🚨 Risk Assessment

### **High-Risk Dependencies**

**IPFS Reliability:**
- **Risk**: IPFS network unavailability or content pinning failures
- **Mitigation**: Use multiple IPFS gateways, implement retry logic, consider backup pinning services

**On-Chain Program Updates:**
- **Risk**: Breaking changes to existing deployed campaigns
- **Mitigation**: Version program instructions (v0 vs v1), maintain backward compatibility

### **Medium-Risk Factors**

**Complexity Management:**
- **Risk**: Over-engineering activation patterns
- **Mitigation**: Start with simple registration arrays, add complexity only as needed

**User Experience:**
- **Risk**: Multi-step deployment process confuses users
- **Mitigation**: Clear CLI output, comprehensive error messages, detailed documentation

### **Low-Risk Assumptions**

**Infrastructure Maturity:**
- Database and RPC abstractions are proven and stable
- CSV compilation and merkle tree generation are well-tested
- Deployment transaction building works reliably

## 🎯 Success Definition

**Phase 1 Complete**: Prism Protocol becomes the first trust-minimized token distribution platform where:
- Campaign creators can cryptographically verify their deployed campaigns match their original CSV data
- Token claimants can independently validate their eligibility without trusting any platform
- Auditors can reproduce entire campaigns locally and verify all deployment targets serve consistent data

**This foundation enables infinite scale, zero-trust automation, and complete elimination of platform dependencies.** 
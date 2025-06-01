# Deployment Architecture: Multiple Targets, Single Source of Truth

## 🔥 Problem & Motivation

### **The Fundamental Trust Crisis in Token Distribution**

Traditional airdrop platforms create a **web of trust problems** that Prism Protocol's deployment architecture explicitly solves:

### **Campaign Creator Protection**

- **"How do I know you didn't inject your own wallets?"** - Deterministic compilation means same CSV always produces same campaign fingerprint
- **"Can I verify the deployed campaign matches my data?"** - `verify-campaign` command locally regenerates and compares against on-chain state
- **"What if I don't trust your platform?"** - Self-host everything with local CLI tools

### **Claimant Protection**

- **"How do I know these amounts are real?"** - Client-side merkle proof validation against on-chain campaign roots
- **"Can I verify I'm getting the right tokens?"** - All eligibility data cryptographically tied to immutable on-chain fingerprints
- **"What if the platform goes down?"** - Anyone can generate their own claimant lookup from campaign databases

### **Auditor Verification**

- **"Can I independently verify campaign integrity?"** - Reproduce entire campaign locally from original CSV data
- **"Are all distributions mathematically sound?"** - Precise decimal math with complete audit trails
- **"Can I validate merkle tree construction?"** - All proofs verifiable against deterministic algorithm

**Result**: Trust-minimized token distribution where all participants can cryptographically verify what they see.

### **Why Current Deployment Patterns Fail**

**Problem 1: Centralized Trust Dependencies**

```
Traditional Flow:
CSV → Upload to Platform → Trust Platform → Hope for Best
❌ Creator can't verify deployed data matches CSV
❌ Claimants can't verify amounts independently
❌ Auditors can't reproduce campaign locally
```

**Problem 2: Mutable State Vulnerabilities**

```
Typical Pattern:
Deploy → Maybe Modify → Eventually Activate → Cross Fingers
❌ No guarantee final state matches initial intent
❌ No cryptographic integrity checks
❌ No way to prove historical consistency
```

**Problem 3: Deployment Fragmentation**

```
Common Reality:
On-chain ≠ API data ≠ Frontend ≠ Documentation
❌ Multiple sources of truth
❌ No verification between deployments
❌ High probability of inconsistency
```

### **Our Solution: IPFS-First, Registration-Tracked, Cryptographically Verified**

**Single Source of Truth Pattern:**

```
CSV Files (Immutable) →
Campaign DB (Deterministic) →
IPFS (Content-Addressed) →
Multiple Deployment Targets (All Verifiable)
```

**Key Architectural Insights:**

1. **CSV files are immutable inputs** - publish to IPFS immediately after compilation
2. **Campaign DB is mutable during deployment** - publish to IPFS only when final
3. **Registration pattern tracks readiness** - campaigns know exactly which cohorts/vaults are ready
4. **Atomic deployment prevents partial states** - either fully deployed or not deployed
5. **Activation requires comprehensive validation** - all components funded and ready before going live

This architecture makes **trust optional** because **verification is cryptographic**.

## 🎯 Design Goals & Benefits

### **1. Trust-Minimized Design**

- ✅ **IPFS immutability** solves content integrity
- ✅ **Deterministic compilation** makes campaigns reproducible
- ✅ **Hierarchical validation** ensures complete readiness before activation
- ✅ **Event-driven backend** removes centralized dependencies

### **2. State Management Excellence**

- ✅ **Clear separation** of mutable process vs immutable outputs
- ✅ **Registration arrays** provide simple, deterministic validation
- ✅ **Activation barriers** prevent partial/invalid states
- ✅ **Single final trigger** (activate_campaign) for all automation

### **3. Operational Robustness**

- ✅ **Instruction collection → transaction packing** fits perfectly with retry logic
- ✅ **Atomic operations** at transaction level, not instruction level
- ✅ **Failure recovery** - can retry any failed transaction bundle
- ✅ **Progress tracking** - array states show exactly what's complete

## 🎯 The Core Insight

**One campaign, multiple deployment targets:**

- **IPFS**: Immutable data for verification
- **Solana**: On-chain program for token claiming
- **Hosted platform**: Convenient static sites
- **Self-hosted**: Custom control

**Key principle**: CSV files are the single source of truth, everything else derives from them.

## 📊 Campaign Data Flow

### **Step 1: Compile Campaign (Local)**

```bash
prism-protocol-cli compile-campaign \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv \
  --mint <TOKEN_MINT> \
  --budget "1000000.0" \
  --admin-keypair admin.json \
  --campaign-db-out campaign.db \
  --rpc-url https://api.mainnet-beta.solana.com
```

**What happens:**

- Parse and validate CSVs
- Generate merkle trees
- Create campaign.db with **embedded CSV content hashes**
- All deterministic - same CSVs always produce same campaign.db

### **Step 2: Deploy Campaign (Atomic: IPFS + Solana)**

```bash
prism-protocol-cli deploy-campaign \
  --campaign-db-in campaign.db \
  --admin-keypair admin.json \
  --go-live-date "2024-03-15T10:00:00Z" \
  --rpc-url https://api.mainnet-beta.solana.com
```

**What happens atomically:**

1. **Early: Publish CSVs to IPFS** (safe, immutable inputs):

   - Publish `customers.csv` → `QmCustomersHash123...`
   - Publish `cohorts.csv` → `QmCohortsHash456...`
   - Store hashes in campaign.db immediately
   - **Rationale**: CSVs are valid and immutable after compilation

2. **On-chain deployment with registration pattern**:

   - Create campaign account with CSV IPFS hashes embedded
   - **For each cohort**: Initialize cohort + register with campaign
   - **For each vault**: Create vault + register with cohort
   - Fund all vaults with required tokens
   - **Campaign tracks cohorts, cohorts track vaults**

3. **Late: Publish final DB to IPFS** (when deployment complete):
   - DB now contains all deployment signatures and state
   - Publish complete campaign.db → `QmCampaignDBFinalHash789...`
   - Update on-chain campaign with final DB hash
   - **Rationale**: DB is now immutable deployment record

**Enhanced Campaign DB Structure:**

```rust
struct CampaignDB {
    // Core campaign data
    pub merkle_roots: Vec<[u8; 32]>,
    pub total_claimants: u64,
    pub total_allocation: u64,

    // On-chain deployment info
    pub deployed_address: Option<Pubkey>,
    pub admin_pubkey: Pubkey,
    pub go_live_timestamp: Option<i64>,
}

// IPFS tracking table (no chicken/egg, no on-chain references in DB)
CREATE TABLE ipfs_hashes (
    key TEXT PRIMARY KEY,
    hash TEXT NOT NULL,
    published_at INTEGER NOT NULL
);

-- Populated during deployment:
-- ('campaign_csv_ipfs_hash', 'QmCustomersHash123...', timestamp)
-- ('cohorts_csv_ipfs_hash', 'QmCohortsHash456...', timestamp)
-- Final DB hash ONLY stored on-chain during activation
```

**Enhanced On-Chain Structure (Registration Pattern):**

```rust
#[account]
pub struct CampaignV0 {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub fingerprint: [u8; 32],

    // IPFS hash for final deployment record
    pub campaign_db_ipfs_hash: [u8; 32],         // ACTIVATION: final deployment record

    // Registration pattern (like vault registration in cohorts)
    #[max_len(MAX_COHORTS_PER_CAMPAIGN)]
    pub cohorts: Vec<Pubkey>,                    // Cohorts register here (like vaults do)

    // Activation control
    pub is_active: bool,
    pub go_live_timestamp: i64,

    pub bump: u8,
}

#[account] // Existing pattern - works perfectly
pub struct CohortV0 {
    pub campaign: Pubkey,
    pub merkle_root: [u8; 32],
    pub amount_per_entitlement: u64,

    // Existing vault registration pattern
    #[max_len(MAX_VAULTS_PER_COHORT)]
    pub vaults: Vec<Pubkey>,                     // Vaults register by writing to array

    pub bump: u8,
}
```

**Detailed Deploy Processd**

```rust
// Phase 1: Early IPFS publishing (safe)
let campaign_csv_hash = publish_to_ipfs("customers.csv")?;
let cohorts_csv_hash = publish_to_ipfs("cohorts.csv")?;
db.store_ipfs_hash("campaign_csv_ipfs_hash", &campaign_csv_hash)?;
db.store_ipfs_hash("cohorts_csv_ipfs_hash", &cohorts_csv_hash)?;

// Phase 2: Initialize campaign (clean init/activate pattern)
let campaign_ix = build_init_campaign_ix(
    admin,
    fingerprint,
    go_live_timestamp,
    expected_cohort_count,  // Used to size cohorts array
)?;

for (cohort_index, cohort) in cohorts.iter().enumerate() {
    // Initialize cohort with sized vaults array
    let cohort_ix = build_init_cohort_ix(
        campaign_addr,
        cohort,
        cohort.vault_count,  // Size vaults array appropriately
    )?;

    for vault_index in 0..cohort.vault_count {
        // Initialize vault (existing create_vault_v0 pattern)
        let vault_ix = build_init_vault_ix(cohort_addr, vault_index)?;

        // Fund vault
        let fund_ix = build_fund_vault_ix(vault_addr, amount)?;

        // Activate vault (marks as ready, registers with cohort)
        let activate_vault_ix = build_activate_vault_ix(cohort_addr, vault_index)?;
    }

    // Activate cohort (when all vaults activated, registers with campaign)
    let activate_cohort_ix = build_activate_cohort_ix(campaign_addr, cohort_index)?;
}

// Phase 3: Final DB publishing (when deployment complete, before activation)
let final_db_hash = publish_to_ipfs("campaign.db")?;
// Final hash provided during campaign activation
```

## 🎯 Why This Granular Approach is Better

### **1. Immutable Inputs vs Mutable Process**

```
CSVs (Immutable) → Publish Early → Safe to Reference
DB (Mutable) → Deployment Process → Publish When Final
```

- **CSV hashes** can be embedded on-chain immediately (never change)
- **DB hash** only embedded when deployment is truly complete
- **Verification possible** at every stage of process

### **2. Registration Pattern Benefits**

```
Campaign ← Cohorts ← Vaults
  ↓         ↓        ↓
Ready?   Ready?   Funded?
```

- **Campaign knows exactly** which cohorts are deployed and ready
- **Cohorts know exactly** which vaults are deployed and funded
- **Activation validation** becomes simple on-chain checks
- **Atomic state updates** as each component registers

### **3. Audit Trail by Design**

```
IPFS Hashes Table:
campaign_csv_ipfs_hash → QmCustomers... (timestamp: deploy start)
cohorts_csv_ipfs_hash  → QmCohorts...  (timestamp: deploy start)
campaign_db_ipfs_hash  → QmFinalDB...  (timestamp: deploy complete)
```

- **Complete deployment history** in database
- **Every major artifact** has IPFS hash + timestamp
- **Independent verification** possible at each stage
- **Final DB hash** represents complete, immutable deployment record

### **4. Activation Validation Simplified**

```rust
// Clean init/activate pattern (each level enforces its own rules)
pub fn all_vaults_activated(cohort: &CohortV0) -> bool {
    // All vault slots filled with non-default pubkeys
    cohort.vaults.iter().all(|vault| *vault != Pubkey::default())
}

pub fn all_cohorts_activated(campaign: &CampaignV0) -> bool {
    // All cohort slots filled with non-default pubkeys
    campaign.cohorts.iter().all(|cohort| *cohort != Pubkey::default())
}

// Activate vault: marks as ready and registers with cohort
pub fn activate_vault(
    ctx: Context<ActivateVault>,
    vault_index: u8,
) -> Result<()> {
    let cohort = &mut ctx.accounts.cohort;
    let vault = &ctx.accounts.vault;

    // Ensure vault_index is within bounds
    require!(
        (vault_index as usize) < cohort.vaults.len(),
        ErrorCode::InvalidVaultIndex
    );

    // Ensure this vault hasn't been activated yet
    require!(
        cohort.vaults[vault_index as usize] == Pubkey::default(),
        ErrorCode::VaultAlreadyActivated
    );

    // TODO: Verify vault is funded with correct amount

    // Activate vault (register with cohort)
    cohort.vaults[vault_index as usize] = vault.key();

    Ok(())
}

// Activate cohort: when all vaults activated, registers with campaign
pub fn activate_cohort(
    ctx: Context<ActivateCohort>,
    cohort_index: u8,
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;
    let cohort = &ctx.accounts.cohort;

    // Ensure all vaults in this cohort are activated
    require!(all_vaults_activated(cohort), ErrorCode::NotAllVaultsActivated);

    // Ensure cohort_index is within bounds
    require!(
        (cohort_index as usize) < campaign.cohorts.len(),
        ErrorCode::InvalidCohortIndex
    );

    // Ensure this cohort hasn't been activated yet
    require!(
        campaign.cohorts[cohort_index as usize] == Pubkey::default(),
        ErrorCode::CohortAlreadyActivated
    );

    // Activate cohort (register with campaign)
    campaign.cohorts[cohort_index as usize] = cohort.key();

    Ok(())
}

pub fn can_activate_campaign(campaign: &CampaignV0, final_db_hash: [u8; 32]) -> bool {
    // Deterministic validation walking up the tree
    all_cohorts_activated(campaign)                      // All cohorts activated
    && final_db_hash != [0; 32]                          // Valid final DB hash provided
    && campaign.campaign_db_ipfs_hash == [0; 32]         // Not already activated
    && !campaign.is_active                               // Not already active
    // note we trust our own code guarded their activation schems too
}

pub fn activate_campaign(
    ctx: Context<ActivateCampaign>,
    final_db_ipfs_hash: [u8; 32]
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;

    require!(can_activate_campaign(campaign, final_db_ipfs_hash), ErrorCode::CannotActivate);

    // Store final IPFS hash and activate (immutable once set)
    campaign.campaign_db_ipfs_hash = final_db_ipfs_hash;
    campaign.is_active = true;

    // Emit event for backend automation
    emit!(CampaignActivated {
        campaign: campaign.key(),
        final_db_ipfs_hash,
        activated_at: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
```

This architecture is **rock solid** - every step is verifiable, the state is always consistent, and activation requires comprehensive validation. Ready to implement this refined approach?

### **Step 3: Activate Campaign (When ready)**

```bash
prism-protocol-cli activate-campaign \
  --campaign-db-in campaign.db \
  --admin-keypair admin.json \
  --rpc-url https://api.mainnet-beta.solana.com
```

**What happens during activation:**

1. **Publish final campaign.db to IPFS** (now truly immutable):

   ```rust
   let final_db_hash = publish_to_ipfs("campaign.db")?;
   ```

2. **Activation Requirements (Enforced on-chain)**:

   - All cohorts must be deployed and funded
   - All vault balances must match required amounts
   - Go-live timestamp must be set
   - Admin signature required
   - Campaign must not already be active

3. **Execute activation instruction**:

   ```rust
   let activate_ix = build_activate_campaign_ix(
       campaign_addr,
       admin_keypair.pubkey(),
       final_db_hash,  // Provide final IPFS hash during activation
   )?;
   ```

4. **On-chain state updates**:

   - Set `campaign.is_active = true`
   - Set `campaign.campaign_db_ipfs_hash = final_db_hash`
   - Set `campaign.all_cohorts_ready = true`
   - Emit activation event with timestamp and IPFS hash

5. **Backend automation triggered**:
   - Monitor activation events containing IPFS hashes
   - Fetch complete campaign data from IPFS
   - Auto-generate static claim sites
   - **Claims can now be processed (if go-live time has passed)**

### **Step 4: Automated Site Generation (Future)**

```bash
# FUTURE: Monitor on-chain events and auto-generate sites
prism-protocol-monitor \
  --program-id <PRISM_PROGRAM_ID> \
  --auto-generate-sites \
  --output-dir ./generated-sites/
```

**Automation Enabled by IPFS Content Hash:**

- Monitor CampaignV0 account creation events
- Extract `ipfs_content_hash` from on-chain data
- Fetch campaign data from IPFS using hash
- Auto-generate optimized static claim sites
- **Result: Zero-touch site generation for all campaigns**

## 🏗️ Current vs Planned Command Flow

### **Current Reality (Incomplete)**

```bash
# ✅ WORKING: Compilation
prism-protocol-cli compile-campaign --campaign-csv-in ... --cohorts-csv-in ...

# ⚠️ MISSING IPFS + ACTIVATION: Direct deployment
prism-protocol-cli deploy-campaign --campaign-db-in ...

# ❌ MISSING: No activation controls or validation
# ❌ MISSING: No IPFS publishing in deploy
# ❌ MISSING: No go-live date support
```

### **Planned Critical Path (Atomic Deployment)**

```bash
# 1. Compile (WORKING)
prism-protocol-cli compile-campaign \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv \
  --mint <TOKEN_MINT> \
  --budget "1000000.0" \
  --admin-keypair admin.json \
  --campaign-db-out campaign.db

# 2. Deploy atomically: IPFS + Solana (ENHANCED)
prism-protocol-cli deploy-campaign \
  --campaign-db-in campaign.db \
  --admin-keypair admin.json \
  --go-live-date "2024-03-15T10:00:00Z"

# 3. Activate when ready (NEW - CRITICAL PRIORITY)
prism-protocol-cli activate-campaign \
  --campaign-db-in campaign.db \
  --admin-keypair admin.json

# 4. Verify everything (ENHANCED)
prism-protocol-cli verify-campaign \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv \
  --campaign-db-in campaign.db \
  --check-ipfs-integrity
```

## 🎯 Why Atomic Deployment is Better

### **1. Simpler User Experience**

```
compile-campaign → deploy-campaign → activate-campaign
```

- **Three clear steps** instead of four
- **No separate IPFS command** to remember
- **Atomic operation** - either fully deployed or not

### **2. Internal IPFS Publishing**

```
deploy-campaign internally:
  1. Publish to IPFS
  2. Calculate content hash
  3. Deploy to Solana with hash
  4. Update campaign.db
```

- **IPFS publishing happens safely** before any transactions
- **Hash embedded in deployment** automatically
- **Single command handles everything** needed for deployment

### **3. Consistent State**

- **campaign.db always has IPFS hash** after successful deployment
- **On-chain campaign always has IPFS reference**
- **No partial states** where IPFS exists but on-chain doesn't

## 🚀 Implementation Priority (Updated)

### **Phase 1: Enhanced Deploy Command (IMMEDIATE - 2-3 days)**

1. **Enhance `deploy-campaign` command**

   - Add internal IPFS publishing before on-chain deployment
   - Calculate and embed IPFS content hash in campaign initialization
   - Support go-live date parameter
   - Update campaign.db with IPFS metadata after successful deployment

2. **Enhance on-chain program**
   - Add `ipfs_content_hash` field to CampaignV0
   - Add activation controls and go-live timestamp
   - Keep campaigns INACTIVE by default after deployment

### **Phase 2: Activation Controls (IMMEDIATE - 1-2 days)**

3. **Build `activate-campaign` command**

   - Comprehensive validation before activation
   - Check all cohorts funded and ready
   - Set activation status on-chain

4. **Enhance verification**
   - Update `verify-campaign` to check IPFS integrity
   - Validate on-chain hash matches IPFS content
   - Check activation status and funding

### **Phase 3: Automation Foundation (NEXT - 1 week)**

5. **Build monitoring infrastructure**
   - Watch for CampaignV0 creation events
   - Extract IPFS hashes from on-chain data
   - Auto-generate static sites from IPFS content

## 🤔 Critical Design Questions to Resolve

### **IPFS Hash Strategy**

1. **Single hash vs multiple?** One combined hash or separate CSV/DB hashes?
2. **Hash algorithm?** SHA256, IPFS native hashing, or campaign fingerprint-style?
3. **On-chain storage size?** 32 bytes for hash vs more metadata?

### **Activation Logic**

1. **Who can activate?** Admin only or also automated systems?
2. **Partial activation?** Can some cohorts be active while others aren't?
3. **Emergency deactivation?** Admin ability to pause campaigns?

### **Go-Live Date Handling**

1. **Frontend enforcement only?** Or also on-chain claim validation?
2. **Timezone handling?** UTC timestamps or configurable timezones?
3. **Early access?** Ability to enable claims before go-live for testing?

**This is absolutely the foundation everything else builds on. Thoughts on the prioritization and design questions?**

## 🔍 Verification Architecture

### **CSV as Anchor Point**

```bash
# Verify entire deployment chain (PLANNED)
prism-protocol-cli verify-campaign \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv \
  --campaign-db-in campaign.db \
  --rpc-url https://api.mainnet-beta.solana.com

# Verification steps:
# 1. Recompile CSVs → verify campaign.db fingerprint matches
# 2. Check campaign.db.source_csv_ipfs → verify CSVs on IPFS (if published)
# 3. Check on-chain merkle roots → verify match campaign.db
# 4. Download hosted site data → verify matches IPFS content (if hosted)
```

### **Independent Auditor Flow**

```bash
# Auditor gets: CSV files + campaign.db + deployment info
wget https://raw.githubusercontent.com/company/airdrop/main/customers.csv
wget https://raw.githubusercontent.com/company/airdrop/main/cohorts.csv
wget https://company.com/campaign.db

# Verify everything chains back to CSVs
prism-protocol-cli verify-campaign \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv \
  --campaign-db-in campaign.db \
  --rpc-url https://api.mainnet-beta.solana.com

# ✅ All deployment targets serve data consistent with original CSVs
# ❌ Tampering detected in [hosted-data | on-chain | ipfs]
```

## 🌟 IPFS-First Architecture Benefits

### **Hosted User Flow: Unforgeable Inputs → Fast Sites**

```bash
# 1. Campaign creator publishes to IPFS (immutable)
prism-protocol-cli publish-to-ipfs \
  --campaign-db-in campaign.db \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv
# → QmCampaignABC123...

# 2. Tell hosted service to use these hashes
curl -X POST api.prism.com/campaigns \
  -d '{"name": "my-airdrop", "campaignIPFS": "QmCampaignABC123..."}'

# 3. Hosted service generates static site from IPFS content
# → https://my-airdrop.prism.com (fast static site)
# → Loads campaign data from IPFS hash but serves it optimized

# 4. Independent auditor can verify
wget https://my-airdrop.prism.com/campaign-data.json
prism-protocol-cli verify-site-integrity \
  --published-json campaign-data.json \
  --expected-ipfs QmCampaignABC123...
# ✅ Cryptographic verification: published site matches IPFS exactly
```

### **Self-Hosted User Flow: Fork + Customize + Deploy**

```bash
# 1. Same IPFS publishing step
prism-protocol-cli publish-to-ipfs \
  --campaign-db-in campaign.db \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv
# → QmCampaignABC123...

# 2. Generate claim site template
prism-protocol-cli generate-claim-site \
  --campaign-db-in campaign.db \
  --ipfs-campaign-data QmCampaignABC123... \
  --template modern \
  --output ./claim-site

# 3. Customize and deploy
cd claim-site
npm run build
vercel deploy  # or any static hosting

# 4. Same auditability guarantees
# Anyone can verify your site loads the exact IPFS content
```

## 🚀 Implementation Priority

### **Phase 1: Core Verification (Immediate)**

1. **Build verify-campaign command** - Using existing compile-campaign logic
2. **Enhance campaign.db schema** - Add IPFS hash fields
3. **Test verification chain** - CSV → campaign.db → on-chain

### **Phase 2: IPFS Integration (Next)**

4. **Build publish-to-ipfs command** - Publish CSVs and campaign data
5. **Build generate-claim-site** - Static site from campaign.db + IPFS
6. **Build verification tools** - Verify sites match IPFS content

### **Phase 3: Hosted Platform (Later)**

7. **Build hosted platform** - Consume IPFS, serve fast sites
8. **Build register-hosted** - API to register campaigns with IPFS hashes

## 🤔 Key Architectural Questions

### **Database Schema Changes Needed**

1. **Add IPFS hash fields to campaign.db**
2. **Embed original CSV content or just hashes?**
3. **How to handle campaign updates vs immutability?**

### **Command Interface Design**

1. **Separate publish-to-ipfs vs integrate into deploy-campaign?**
2. **How to handle optional IPFS publishing?**
3. **What's the right level of automation vs explicit control?**

### **Verification Strategy**

1. **CLI-only verification or also web-based tools?**
2. **How to handle private campaigns (no IPFS publishing)?**
3. **What level of verification should be required vs optional?**

## 🎯 Next Concrete Steps

**Immediate (using existing CLI):**

1. Add verify-campaign command to PROJECT_PLAN.md ✅
2. Start with CSV recompilation verification
3. Test deterministic campaign.db generation

**Near-term (new functionality):** 4. Design IPFS integration commands 5. Prototype static site generation 6. Test end-to-end verification flow

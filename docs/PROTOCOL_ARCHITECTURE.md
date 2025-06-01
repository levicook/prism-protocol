# Protocol Architecture: Trust-Minimized Token Distribution

## 🎯 Platform Guarantees

### **What Prism Protocol Promises**

**For Campaign Creators:**
- ✅ **Deployed campaign exactly matches your CSV data** - Deterministic compilation with integrated IPFS publishing creates verifiable chain from source to deployment
- ✅ **No hidden modifications possible** - IPFS content addressing prevents tampering, on-chain commitments make modifications detectable  
- ✅ **Independent verification available** - Anyone can reproduce your campaign locally and verify against IPFS + on-chain references
- ✅ **Complete audit trail preserved** - Every step from CSV to final deployment recorded with IPFS content hashes and cryptographic integrity

**For Token Recipients:**
- ✅ **Amounts are cryptographically guaranteed** - Your allocation exists in IPFS merkle trees, verified against immutable on-chain commitments
- ✅ **No platform dependency for claiming** - Claims work directly from IPFS data using on-chain reference points, platforms optional
- ✅ **Transparent eligibility verification** - Anyone can verify you're getting exactly what's in the original IPFS-published CSV
- ✅ **Self-sovereign claim verification** - Generate and validate your own proofs from IPFS data without trusting any service

**For Auditors:**
- ✅ **Complete campaign reproducibility** - Same CSV inputs always generate same IPFS content hashes and on-chain fingerprints
- ✅ **Mathematical verification of distributions** - Every allocation traceable from IPFS sources through cryptographic proofs  
- ✅ **Independent platform verification** - Audit any deployment target against authoritative IPFS sources
- ✅ **Immutable deployment history** - IPFS content addressing creates unforgeable audit trails with on-chain timestamps

### **How These Guarantees Work**

## 🏗️ Architecture Overview

### **Single Source of Truth Pattern**

```
CSV Files (Human-Readable) 
    ↓ [Deterministic Compilation + Integrated IPFS Publishing]
Campaign Database (Machine-Readable) with Embedded IPFS References
    ↓ [Multi-Target Deployment: IPFS + Solana + Platforms]
IPFS Network (Immutable Content Layer) ←→ Solana Program (On-Chain State)
    ↓ [Automatic Discovery & Verification]
┌─ Hosted Platforms (Fast UX)
├─ Self-Hosted Sites (Custom Control)  
├─ API Services (Integration Layer)
└─ Verification Tools (Independent Audit)
```

**Key Insight**: IPFS and Solana are co-equal infrastructure layers. One immutable IPFS-published input feeds multiple deployment targets, all cryptographically verifiable against the original data.

### **Trust-Minimized Design Principles**

#### **1. Deterministic Compilation with Integrated Publishing**
- Same CSV files always produce identical merkle trees
- **CSV files immediately published to IPFS during compilation**
- Campaign database contains IPFS content hashes as canonical references
- Same fingerprints always generate identical on-chain state
- **Result**: Campaign creators can verify deployment matches their intent, with immutable source preservation

#### **2. IPFS-First Content Architecture**  
- All campaign data flows through IPFS as primary content layer
- Content hashes embedded in on-chain state create immutable cross-references
- Multi-target deployment uses IPFS as single source of truth
- **Result**: No party can modify data without detection, automatic platform discovery

#### **3. Cryptographic Commitment Across Layers**
- **IPFS contains all verifiable content**: Original CSVs, merkle trees, recipient lists, complete campaign data
- **On-chain contains cryptographic commitments**: Merkle roots that allow validation of IPFS content
- **Verification works by proving IPFS data against on-chain commitments**
- Cross-layer verification ensures end-to-end integrity
- **Result**: IPFS data is cryptographically verifiable using on-chain reference points

#### **4. Multi-Target Hierarchical Verification**
- **IPFS verification** → CSV verification → Database verification → On-chain commitment validation
- **IPFS contains the actual data needed for verification** (merkle trees, proofs, original sources)
- **On-chain provides immutable reference points** (roots) that make IPFS data verifiable
- Each deployment target provides independent validation against IPFS sources
- Breaks in any verification chain are immediately detectable
- **Result**: Comprehensive audit trail where IPFS provides content and on-chain provides commitments

## 📊 Protocol Flow

### **Phase 1: Compilation with Integrated IPFS Publishing**

```bash
prism-protocol-cli compile-campaign \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv \
  --mint <TOKEN_MINT> \
  --admin-keypair admin.json \
  --campaign-db-out campaign.db
```

**What Happens Atomically:**
1. Parse and validate all recipient data
2. Generate deterministic merkle trees with vault assignments  
3. **Immediately publish CSV files to IPFS**:
   - `customers.csv` → `QmCustomers123...`
   - `cohorts.csv` → `QmCohorts456...`
4. **Store IPFS hashes in campaign database**:
   - `campaign.db.csv_hashes.customers = "QmCustomers123..."`
   - `campaign.db.csv_hashes.cohorts = "QmCohorts456..."`
5. Calculate campaign fingerprint that includes IPFS content hashes

**Guarantees**: 
- Same CSV inputs always produce same campaign.db with same fingerprint
- **Original data is immediately preserved immutably on IPFS**
- **Campaign database contains cryptographic references to original sources**

### **Phase 2: Multi-Target Deployment**

```bash
prism-protocol-cli deploy-campaign \
  --campaign-db-in campaign.db \
  --admin-keypair admin.json \
  --go-live-date "2024-03-15T10:00:00Z"
```

**Multi-Target Deployment Happens Simultaneously:**

#### **Target 1: Solana Program (On-Chain State)**
- Deploy campaign account with embedded CSV IPFS hashes
- Create all required token vaults with deterministic addresses
- Fund vaults with exact amounts specified in original CSVs
- Activate campaign only when all components verified

#### **Target 2: IPFS Network (Immutable Data Layer)**
- **Publish complete campaign.db to IPFS** → `QmCampaignDB789...`
- Pin campaign data across IPFS network for redundancy
- Create content-addressed reference for all deployment targets
- **On-chain state references this final IPFS hash**

#### **Target 3: Platform Integration (UX Layer)**
```bash
# Hosted platforms automatically discover new campaigns via IPFS
# No separate API call needed - they monitor IPFS for new campaign hashes
```

- Platforms monitor IPFS for new campaign deployments
- Fetch campaign data directly from `QmCampaignDB789...`
- Generate optimized claim interfaces automatically
- All data cryptographically tied to original CSV sources

#### **Target 4: Self-Hosted Deployment (Decentralized UX)**
```bash
# Anyone can immediately deploy claim sites from IPFS data
prism-protocol-cli generate-claim-site \
  --campaign-ipfs-hash QmCampaignDB789... \
  --template modern \
  --output ./my-claim-site
```

**Deployment Flow Summary:**
```
compile-campaign → [CSV files immediately on IPFS] 
                ↓
deploy-campaign → [Solana + IPFS + Platform Discovery + Self-Host Ready]
                ↓
activate-campaign → [All targets go live simultaneously]
```

### **Phase 3: Independent Verification**

```bash
# Anyone can verify the entire multi-target deployment
prism-protocol-cli verify-campaign \
  --campaign-ipfs-hash QmCampaignDB789... \
  --check-all-targets
```

**Verification Steps Across All Targets:**
1. **IPFS → CSV Sources**: Fetch campaign.db from IPFS, verify CSV hashes match original files
2. **CSV → Database**: Recompile CSVs locally, verify fingerprint matches
3. **Database → On-Chain**: Verify on-chain merkle roots match database content
4. **On-Chain → Reality**: Verify vault funding matches expected amounts
5. **Cross-Target Consistency**: Verify all deployment targets serve identical data

**Guarantee**: Complete verification across all deployment targets without trusting any party.

## 🔒 Security Model

### **What You Must Trust**
- **IPFS content addressing** - Standard distributed systems assumption for content integrity
- **Solana blockchain integrity** - Standard crypto assumption for immutable commitments
- **Your own local verification** - You run the verification against IPFS data yourself

### **What You Don't Need to Trust**
- ❌ **Prism Protocol service availability** - IPFS data and on-chain commitments work without our platform
- ❌ **Campaign creator honesty** - IPFS contains cryptographic proofs verifiable against on-chain commitments
- ❌ **Hosted platform integrity** - Independent verification against IPFS detects tampering
- ❌ **API service correctness** - All data verifiable against IPFS sources using on-chain reference points

### **How Verification Actually Works**

**The Key Insight**: Neither IPFS nor on-chain alone provides verification - they work together:

1. **IPFS contains verifiable content**:
   - Original CSV files with complete recipient data
   - Merkle trees with all proofs and leaves
   - Campaign database with complete distribution logic
   - All the actual data needed to verify claims

2. **On-chain contains immutable commitments**:
   - Merkle roots that commit to IPFS merkle trees
   - Campaign fingerprints that commit to IPFS data integrity
   - Vault addresses that must match IPFS vault assignments
   - Reference points that make IPFS data cryptographically verifiable

3. **Verification proves IPFS data against on-chain commitments**:
   - Download merkle tree from IPFS → Verify root matches on-chain commitment
   - Download recipient list from IPFS → Verify inclusion proofs against on-chain root
   - Download original CSVs from IPFS → Verify they produce same merkle trees
   - **Result**: IPFS data is cryptographically proven authentic

### **Attack Resistance**

#### **Data Tampering Resistance**
- Campaign creator can't modify after deployment: **On-chain commitments are immutable**
- Platform can't inject fake recipients: **All proofs verify against IPFS data using on-chain roots**
- Hosted services can't modify amounts: **IPFS content hashes detect any changes**
- **Key**: Tampering requires modifying both IPFS content AND on-chain commitments (impossible)

#### **Verification Bypass Resistance**
- Can't skip IPFS verification: **On-chain commitments are meaningless without IPFS content**
- Can't skip on-chain verification: **IPFS data is unverifiable without commitment references**
- Can't skip cross-layer verification: **Both layers required for cryptographic proof**
- **Key**: Security depends on IPFS + on-chain integrity, not either alone

## 🚀 Platform Integration Patterns

### **Hosted Platform Integration**

```javascript
// Platform fetches campaign data trustlessly
const campaignData = await fetchFromIPFS(campaign.ipfs_hash);
const merkleRoots = await fetchFromSolana(campaign.address);

// Verify consistency before serving to users
assert(campaignData.merkle_roots === merkleRoots);

// Generate claim interface with embedded verification
const claimSite = generateSite({
  data: campaignData,
  verification: {
    originalCSV: campaign.csv_ipfs_hash,
    onChainRoots: merkleRoots
  }
});
```

**User Experience**: Fast, hosted claim sites with built-in verification tools.
**Trust Model**: Users can verify the hosted site serves correct data.

### **Self-Hosted Integration**

> **⚠️ PLACEHOLDER**: Self-hosted deployment workflow details are not yet finalized. The CLI commands and integration patterns shown below are conceptual and need further design work.

```bash
# Complete independence from any service
git clone campaign-templates
prism-protocol-cli generate-claim-site \
  --campaign-db-in my-campaign.db \
  --template ./campaign-templates/modern
  
# Deploy anywhere
vercel deploy ./claim-site
# or netlify deploy
# or aws s3 sync
```

**User Experience**: Complete control over claim interface and hosting.
**Trust Model**: Zero dependency on any third-party service.

### **API Integration**

```javascript
// Build custom integration consuming IPFS data
const api = new PrismProtocolAPI({
  campaignIPFS: "QmCampaign123...",
  rpcUrl: "https://api.mainnet-beta.solana.com"
});

// All data verifiable against sources
const proof = await api.generateProof(walletAddress);
const isValid = await api.verifyProof(proof);
```

**User Experience**: Custom applications with full protocol integration.
**Trust Model**: All API responses verifiable against cryptographic sources.

## 🎯 Verification Workflows

### **Campaign Creator Verification**

```bash
# Verify your deployment matches your intent
prism-protocol-cli verify-campaign \
  --campaign-csv-in my-customers.csv \
  --cohorts-csv-in my-cohorts.csv \
  --campaign-db-in deployed-campaign.db
  
# ✅ Deployment exactly matches CSV data
# ✅ All merkle roots correctly computed  
# ✅ All vault addresses properly funded
# ✅ IPFS hashes correctly reference original data
```

### **Recipient Verification**

```bash
# Verify your eligibility and amount
prism-protocol-cli verify-recipient \
  --wallet-address <YOUR_WALLET> \
  --campaign-csv-in original-customers.csv \
  --campaign-db-in campaign.db
  
# ✅ You are eligible for X tokens
# ✅ Amount matches original CSV specification
# ✅ Merkle proof validates against on-chain roots
# ✅ No tampering detected in distribution data
```

### **Auditor Verification**

```bash
# Complete independent audit workflow
wget https://company.com/airdrop/customers.csv
wget https://company.com/airdrop/cohorts.csv
wget https://company.com/airdrop/campaign.db

# Reproduce everything locally
prism-protocol-cli compile-campaign \
  --campaign-csv-in customers.csv \
  --cohorts-csv-in cohorts.csv \
  --admin-keypair auditor.json \
  --campaign-db-out audit.db

# Verify consistency
diff campaign.db audit.db  # Should be identical

# Verify on-chain deployment
prism-protocol-cli verify-campaign \
  --campaign-csv-in customers.csv \
  --campaign-db-in campaign.db \
  --check-on-chain \
  --check-vault-funding
```

**Result**: Complete confidence in distribution integrity without trusting any party.

## 🔮 Protocol Evolution

### **Current Capabilities**
- ✅ **Deterministic compilation** from CSV to on-chain state
- ✅ **Multi-target deployment** with cryptographic consistency
- ✅ **Independent verification** of entire distribution chain
- ✅ **Trust-minimized claiming** via merkle proof validation

### **Future Enhancements**

#### **Enhanced IPFS Integration**
- Automatic IPFS publishing during deployment
- Distributed pinning for redundancy
- Content-addressed verification tools

#### **Advanced Verification Tools**
- Real-time monitoring of campaign integrity
- Automated fraud detection systems
- Compliance reporting with cryptographic proofs

#### **Platform Ecosystem**
- Standardized integration APIs
- Plug-and-play claim site templates
- Cross-platform verification tools

---

**The core promise**: Any party can independently verify that deployed campaigns exactly match the original CSV data, making trust optional because verification is cryptographic. 
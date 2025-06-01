# Prism Protocol User Journeys & Trust Model

## 🎯 Overview

This document maps out user journeys and trust relationships to identify verification mechanisms needed for both hosted and self-hosted scenarios.

**Core Trust Challenge**: How do users verify that data serving (hosted or self-hosted) hasn't been tampered with?

## 🎯 Simplified User Journeys (IPFS-First)

### **Campaign Creator: "Publish Unforgeable Data"**

```
1. CSV → CLI validate → CLI publish to IPFS → Get immutable hash
2. Choose: hosted service OR self-host
3. Site generation happens from unforgeable IPFS inputs
4. Auditors can verify site matches original IPFS content
```

### **Token Claimant: "Fast Static Site Experience"**

```
1. Visit normal URL: https://my-airdrop.prism.com
2. Fast static site loads (not ipfs.io URLs)
3. Site loads campaign data from IPFS hash (verified)
4. Connect wallet and claim (normal UX)
```

### **Auditor: "Verify Published Site Against IPFS"**

```bash
# Download what the site is serving
wget https://my-airdrop.prism.com/campaign-data.json

# Verify it matches the claimed IPFS content
prism-protocol-cli verify-site-integrity \
  --site-data campaign-data.json \
  --claimed-ipfs QmCampaignABC123...

# If verification passes → site is serving authentic data
# If verification fails → tampering detected
```

## 🌟 Value Propositions by User Type

### **For Campaign Creators**

- **Unforgeable inputs** - IPFS content addressing prevents tampering
- **Choice of hosting** - Use our hosted service OR self-host
- **Independent auditability** - Anyone can verify integrity
- **Fast user experience** - Static sites, not IPFS gateway dependencies

### **For Token Claimants**

- **Normal website experience** - No scary ipfs.io URLs
- **Cryptographic guarantees** - Site data verified against IPFS
- **Fast loading** - Optimized static sites, not IPFS gateway speed
- **Always verifiable** - Can independently check data authenticity

### **For Auditors**

- **Complete verification** - Can prove site serves authentic IPFS content
- **Independent tooling** - Compile our tools yourself, verify everything
- **Public evidence** - IPFS hashes are permanent, unforgeable evidence

### **For Frontend Builders**

- **Template or custom** - Use our templates or build with tiny SDK
- **IPFS-native** - Content loading with cryptographic verification built-in
- **Framework agnostic** - Works with React, Vue, Svelte, vanilla JS

### **For Us (Business Model)**

- **Clear value add** - We provide fast site generation from unforgeable inputs
- **Can't be "rug pulled"** - All data on IPFS, users own their content
- **Auditability builds trust** - Independent verification possible

## 🪄 User Personas

### **Campaign Creator**

- Wants to distribute tokens to specific recipients
- Needs to ensure only intended recipients can claim
- Must trust that service provider hasn't injected additional claimants

### **Token Claimant**

- Wants to discover eligible campaigns
- Needs to verify eligibility amounts are correct
- Must trust that displayed entitlements match on-chain reality

### **Frontend Builder**

- Wants to build claimant interfaces
- Needs reliable access to campaign/claimant data
- Must choose between hosted service vs self-hosting

### **Auditor/Verifier**

- Wants to verify campaign integrity
- Needs to validate that deployed campaigns match original intent
- Must be able to reconstruct and verify merkle trees independently

## 🔍 Campaign Creator Journey: Trust but Verify

### **Scenario**: Company wants to airdrop tokens to 10,000 customers

### **Detailed Journey (High-stakes, Multi-phase)**

#### **Phase 1: Environment Setup** (First-time only, complex)

```
1. Install our CLI → How? npm? cargo install? Binary download?
2. Install Solana CLI → Which version? How do they know?
3. Generate admin keypair → solana-keygen new admin.json
4. Fund admin keypair → How much SOL needed? Where do they get it?
5. Learn the workflow → Documentation, examples, gotchas
```

#### **Phase 2: CSV Development** (Outside our scope, affects UX)

```
6. Build recipient lists → Hard work, data analysis, business logic
7. Validate CSV format → Our tooling can help here
8. Test with small datasets → Need good testing workflow
```

#### **Phase 3: Campaign Deployment** (High-stakes, bulletproof)

```
9. Compile campaign locally → prism-protocol-cli compile-campaign
10. Verify compilation → Check amounts, fingerprints, etc.
11. Deploy to our on-chain program → Trust required, but verified build
12. Verify deployment succeeded → Campaign is live on-chain
```

#### **Phase 4: Claim Site Generation** ("Deploy to Vercel" vision)

```
13. Generate static claim site → prism-protocol-cli generate-claim-site
14. Customize templates/branding → Edit templates, add logos, etc.
15. Test claim site locally → Verify wallet connection, eligibility checks
16. Deploy to hosting → vercel deploy, netlify, etc.
```

#### **Phase 5: Campaign Lifecycle Management** (Timeline control)

```
17. Publish "check eligibility" mode → Site live, but no claiming yet
18. Campaign activation on date → On-chain campaign.is_active = true
19. Monitor claims progress → Dashboards, alerts, support
20. Handle edge cases → Stuck transactions, user support
```

### **Architectural Principles**

- **Work in Rust as much as possible** - Leverage existing Rust static site generators
- **Easy customization + tiny client SDK** - Minimal JavaScript, maximum flexibility
- **Trust-maximizers compile from source** - Provide clear source compilation path
- **Self-hosting expects tech chops** - Docker deployment, some technical knowledge required
- **Desktop app potential** - For users who want maximum control
- **Full journey documentation** - Including token creation, end-to-end tutorials

### **Critical UX Requirements**

**High Stakes Nature:**

- Real money, real users, public campaigns
- One-shot deployment (hard to fix mistakes)
- Timeline pressure (specific launch dates)
- CSV development already represents significant investment

**Tooling Gaps Identified:**

- Setup automation (environment setup)
- Validation tooling (catch errors early)
- Testing workflow (safe pipeline testing)
- Verification tools (deployment confidence)
- Lifecycle management (phased rollout)
- Documentation/tutorials (full complexity coverage)
- Support/monitoring (error handling)

#### **Step 1: Data Preparation**

```
Campaign Creator: "I have customers.csv with 10,000 wallet addresses"
```

**Questions:**

- How do they know their CSV won't be modified?
- What verification do they get that their exact list was used?

#### **Step 2: Campaign Compilation**

**Hosted Platform Flow:**

```
1. Upload customers.csv to platform
2. Platform compiles campaign → generates merkle tree
3. Platform deploys campaign with fingerprint ABC123
4. Campaign Creator receives: "Campaign deployed at fingerprint ABC123"
```

**Trust Gap**: How does Campaign Creator verify that fingerprint ABC123 corresponds exactly to their original customers.csv?

**Self-Hosted Flow:**

```
1. Campaign Creator runs: prism-protocol-cli compile-campaign --csv customers.csv
2. CLI generates campaign.db with fingerprint ABC123
3. Campaign Creator runs: prism-protocol-cli deploy-campaign --db campaign.db
4. Campaign Creator controls entire process
```

**Trust**: Campaign Creator has full control and can verify every step locally.

#### **Step 3: Verification Requirements**

**What Campaign Creator Needs:**

- **Deterministic compilation**: Same CSV always produces same fingerprint
- **Reproducible builds**: Can regenerate campaign locally to verify fingerprint
- **Audit trail**: Can verify deployed campaign matches their original intent
- **Claimant verification**: Can check that only intended recipients are eligible

### **Proposed Verification Mechanism**

```bash
# Campaign Creator can always verify locally
prism-protocol-cli verify-campaign \
  --original-csv customers.csv \
  --deployed-fingerprint ABC123 \
  --rpc-url https://api.mainnet-beta.solana.com

# Output:
✅ Fingerprint ABC123 matches customers.csv exactly
✅ On-chain merkle roots correspond to provided CSV
✅ No additional claimants detected
✅ All amounts match expected distribution
```

## 🪙 Token Claimant Journey: Eligibility Discovery

### **Scenario**: User hears about airdrop, wants to check and claim

### **Simple User Journey (Should be effortless)**

#### **Phase 1: Discovery & Connection**

```
1. Hear about campaign → Social media, discord, email
2. Visit claim site → https://company-airdrop.vercel.app
3. Connect wallet → Phantom, Solflare, etc. (one click)
4. See eligibility status → Instant feedback
```

#### **Phase 2: Pre-claim Period (Eligibility check only)**

```
5. View entitled amounts → "You're eligible for 500 TOKENS across 2 cohorts"
6. Understand timeline → "Claims go live on March 15th"
7. Prepare for claiming → Ensure wallet has SOL for fees
```

#### **Phase 3: Claiming Period (Claims active)**

```
8. Return to site → When claims are live
9. Review claim details → Final amounts, gas fees
10. Execute claim → Sign transaction(s)
11. Confirmation → Tokens in wallet, receipt stored
```

### **Critical Claimant Requirements**

**Effortless Experience:**

- No CLI tools, no technical knowledge
- Wallet connection works on mobile
- Clear error messages ("Need 0.01 SOL for fees")
- Works across different wallets/devices

**Trust & Verification:**

- Can verify amounts are correct (merkle proof validation)
- Can see campaign source/deployer
- Can verify transaction success independently

**Edge Cases:**

- Multiple cohort eligibility (batch claiming?)
- Failed transactions (retry mechanism)
- Wallet switching (different device/wallet)

#### **Step 1: Campaign Discovery**

```
Claimant: "I heard about XYZ campaign, am I eligible?"
```

**Information Needed:**

- What campaigns exist?
- Which campaigns am I eligible for?
- How much can I claim from each?

#### **Step 2: Eligibility Verification**

**Hosted Platform Flow:**

```
1. Visit hosted lookup: https://claims.prism.com/campaigns
2. Connect wallet: 7BgBvyjrZX8YKHGoW9Y8929nsq6TsQANzvsGVEpVLUD8
3. See eligible campaigns and amounts
```

**Trust Gap**: How does claimant know the displayed amounts are accurate?

**Self-Hosted Flow:**

```
1. Campaign Creator provides their own lookup URL: https://xyz-company.com/claims
2. Claimant connects wallet
3. See campaigns hosted by that specific company
```

**Trust**: Claimant trusts the campaign creator directly.

#### **Step 3: Amount Verification**

**What Claimant Needs:**

- **On-chain verification**: Displayed amounts match merkle proof math
- **Proof validation**: Can verify merkle proof against on-chain root
- **Source verification**: Can see which entity compiled/deployed campaign

### **Proposed Verification Mechanism**

```typescript
// Frontend always validates proofs client-side
const validateEligibility = async (
  claimant: PublicKey,
  proofs: MerkleProof[]
) => {
  // 1. Fetch on-chain campaign and merkle roots
  const onChainCampaign = await connection.getAccountInfo(campaignAddress);

  // 2. Verify merkle proofs against on-chain roots
  for (const proof of proofs) {
    const isValid = verifyMerkleProof(
      proof.leaf,
      proof.proof,
      onChainCampaign.merkleRoot
    );
    assert(isValid, "Merkle proof validation failed");
  }

  // 3. Verify amounts match proof calculations
  const calculatedAmount = proof.entitlements * proof.amountPerEntitlement;
  assert(calculatedAmount === proof.totalClaimable, "Amount mismatch");

  return true; // All verifications passed
};
```

## 🛠️ Frontend Builder Journey: Data Access

### **Scenario**: Team wants to build custom claiming interface

### **Developer Journey (Should be straightforward)**

#### **Phase 1: Project Setup**

```
1. Choose integration approach → Use our template vs. build from scratch
2. Install dependencies → Our tiny client SDK + Solana wallet adapters
3. Set up development environment → Next.js, Vite, whatever they prefer
4. Get sample data → Test campaigns for development
```

#### **Phase 2: Development**

```
5. Integrate wallet connection → Standard Solana wallet patterns
6. Query eligibility data → Our client SDK handles this
7. Build claim UX → Custom design with our SDK
8. Test end-to-end → Local validator, test campaigns
```

#### **Phase 3: Data & Deployment**

```
9. Get campaign data → Static files or API endpoints
10. Configure production → Real campaign fingerprints, mainnet
11. Deploy frontend → Their hosting choice
12. Monitor & support → Handle user issues
```

### **Frontend Builder Options**

#### **Option 1: Use Our Template** (Recommended)

```
Frontend Builder: "I'll start with your template and customize"
```

**Benefits:**

- Working wallet integration out of the box
- Proven claim flow and error handling
- Easy customization through templates/CSS
- Built-in verification and security

**Process:**

```bash
# Generate customized claim site
prism-protocol-cli generate-claim-site \
  --campaign-db my-campaign.db \
  --template modern \
  --branding-kit ./my-branding/ \
  --output ./my-claim-site

# Customize and deploy
cd my-claim-site
npm run build  # or just static files
vercel deploy   # or any hosting
```

#### **Option 2: Build Custom with Our SDK**

```
Frontend Builder: "I'll build from scratch with your SDK"
```

**Benefits:**

- Full control over UI/UX
- Can integrate into existing sites
- Custom business logic
- Any framework (React, Vue, Svelte)

**Process:**

```typescript
// Install our lightweight client SDK
npm install @prism-protocol/client

// Use in any frontend framework
import { PrismClient, verifyEligibility } from '@prism-protocol/client';

const client = new PrismClient({
  campaignData: '/static/campaign-data.json' // Or API endpoint
});

const eligibility = await client.checkEligibility(walletPubkey);
```

#### **Option 3: Self-Host Data Generation**

```
Frontend Builder: "I'll generate my own data serving"

# Generate static lookup files
prism-protocol-cli generate-claimant-lookup \
  --campaigns-dir ./campaigns \
  --output-dir ./static-lookup

# Serve via own infrastructure
nginx -s ./static-lookup/
```

**Benefits:**

- Full control over data
- No external dependencies
- Can verify everything locally

**Costs:**

- Infrastructure management
- Need to stay synced with campaigns

### **SDK Design Principles**

**Tiny Client SDK:**

- No bundler lock-in (works with any build system)
- Tree-shakeable (only import what you need)
- TypeScript-first with great DX
- Wallet-agnostic (works with any Solana wallet)

**Easy Customization:**

- Template system with clear override points
- CSS custom properties for theming
- Component-level customization
- Clear documentation for common modifications

## 🔍 Auditor/Verifier Journey: Independent Verification

### **Scenario**: Security researcher wants to verify campaign integrity

### **Technical User Journey (CLI-focused)**

#### **Phase 1: Campaign Discovery**

```
1. Learn about campaign → Public announcement, social media
2. Get original data → CSV files, campaign details from creator
3. Identify on-chain deployment → Campaign fingerprint, admin pubkey
4. Set up verification environment → Our CLI, Solana CLI
```

#### **Phase 2: Independent Verification**

```
5. Reproduce campaign compilation → Same CSV + CLI = same fingerprint?
6. Verify on-chain deployment → Fingerprint matches deployed campaign?
7. Check merkle tree integrity → All proofs mathematically valid?
8. Validate fund allocation → Vaults funded correctly?
```

#### **Phase 3: Public Verification**

```
9. Document findings → Report discrepancies or confirm integrity
10. Publish verification → Social proof for community
11. Monitor ongoing claims → Watch for anomalies
```

### **Auditor Tooling Requirements**

**Deterministic Verification:**

```bash
# Must produce identical results to original deployment
prism-protocol-cli verify-campaign \
  --original-csv community-airdrop.csv \
  --deployed-fingerprint abc123... \
  --rpc-url https://api.mainnet-beta.solana.com

# Should output:
✅ CSV hash: abc123... (matches public commitment)
✅ Campaign fingerprint: def456... (matches on-chain)
✅ Merkle roots: ghi789... (all cohorts verified)
✅ Vault funding: 1,000,000 tokens (exact match)
✅ No additional claimants detected
```

**Mathematical Verification:**

```bash
# Verify every merkle proof can be reconstructed
prism-protocol-cli audit-merkle-tree \
  --campaign-db community-airdrop.db \
  --sample-size 1000  # Check random sample

# Verify on-chain state matches expectations
prism-protocol-cli audit-on-chain-state \
  --campaign-fingerprint abc123... \
  --admin-pubkey def456...
```

**Public Reporting:**

- Generate verification reports
- Export audit logs and evidence
- Create reproducible verification instructions
- Document any discrepancies found

### **Trust Model for Auditors**

**What Auditors Verify:**

- Original CSV data integrity (if publicly committed)
- Deterministic compilation (same inputs = same outputs)
- On-chain deployment correctness (campaign matches CSV)
- Mathematical soundness (merkle trees, allocations)
- No injection attacks (extra claimants, inflated amounts)

**What Auditors Provide:**

- Independent verification of campaign integrity
- Public trust signals for the community
- Detection of systemic issues or attacks
- Competitive verification (multiple auditors can check same campaign)

## 🔐 Trust Model Summary

### **Hosted Platform Trust Requirements**

1. **Campaign Creators** must be able to verify:

   - Their exact CSV was used (no injected claimants)
   - Deployed fingerprint matches their original data
   - On-chain state corresponds to their intent

2. **Claimants** must be able to verify:

   - Displayed amounts match on-chain merkle proofs
   - Merkle proofs are mathematically valid
   - Campaign was deployed by legitimate entity

3. **Frontend Builders** must be able to verify:
   - Served data matches on-chain state
   - Service availability and reliability
   - Fallback to self-hosting if needed

### **Self-Hosted Trust Model**

1. **Campaign Creators**: Full control, can verify every step locally
2. **Claimants**: Same cryptographic verification requirements
3. **Frontend Builders**: Full control over data serving and verification

## 📋 Next Steps

**Key Questions to Resolve:**

1. What verification tooling do we need to build?
2. How do we make local verification simple for non-technical users?
3. What's the simplest way to enable self-hosting?
4. How do we handle campaign discovery across multiple hosts?

**Priority Order:**

1. Build robust local verification tools
2. Design hosted platform with verification-first approach
3. Enable simple self-hosting workflows
4. Create frontend integration patterns

## 🏗️ Self-Hosted Template Architecture (VERY SPECULATIVE, MODEL MIGHT BE WRONG, KEEPING AS FOOD FOR THOUGHT)

### **Template Structure (Next.js/React Example)**

```typescript
// Next.js/React template that loads from IPFS
export default function ClaimSite() {
  const campaignIPFS = process.env.CAMPAIGN_IPFS;

  // Load verified campaign data
  const { data: campaignData } = useSWR(campaignIPFS, async (hash) => {
    const response = await fetch(`https://ipfs.io/ipfs/${hash}`);
    const data = await response.json();

    // Verify hash matches content (cryptographic guarantee)
    const actualHash = await calculateIPFSHash(data);
    if (actualHash !== hash) {
      throw new Error("IPFS content verification failed");
    }

    return data;
  });

  return <ClaimInterface campaignData={campaignData} />;
}
```

### **Configuration-Based Workflow**

```bash
# 1. Fork template repository
git clone https://github.com/prism-protocol/claim-site-template
cd claim-site-template

# 2. Configure with your IPFS hashes
echo 'CAMPAIGN_IPFS="QmCampaignABC123..."' > .env

# 3. Customize branding (optional)
cp ./branding-kit/* ./public/branding/

# 4. Deploy anywhere
npm run build
vercel deploy  # or netlify, github pages, etc.
```

## 🤔 Technical UX Considerations

### **IPFS Integration Challenges**

- **Custom domains**: Require IPFS gateway or DNS setup
- **Performance**: Can be slower than dedicated CDN
- **User familiarity**: ipfs.io URLs less familiar than custom domains
- **Gateway reliability**: Public gateways can be inconsistent

### **Hybrid Approach: IPFS + Static Sites**

**Best of both worlds:**

- **IPFS**: Immutable, content-addressed, decentralized data integrity
- **Static Sites**: Fast, familiar, normal website UX for end users
- **Auditability**: Anyone can verify sites serve authentic IPFS content

### **Key UX Questions to Explore**

- How fast is IPFS content loading for end users?
- Which IPFS gateways are most reliable?
- How do we handle IPFS pinning for persistence?
- Can we make IPFS URLs feel like normal websites?
- Are ipfs.io URLs too scary for normal users?
- How do we handle custom domains over IPFS?
- What happens if IPFS gateways are slow/down?
- Can we make wallet connection work smoothly with IPFS sites?

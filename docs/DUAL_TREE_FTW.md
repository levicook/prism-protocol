# 🚀 **Prism Protocol: Dual-Tree Architecture Implementation Plan**

## 📋 **Overview**

Implement **binary trees (V0)** and **256-ary trees (V1)** side-by-side to support both small campaigns (<4K claimants) and massive campaigns (millions of claimants).

### **Key Principles:**

- ✅ **Zero breaking changes** - all existing code stays functional
- 🔄 **Parallel implementation** - V0 and V1 coexist
- 🧪 **Comprehensive testing** - compare performance across both approaches
- 📈 **Gradual migration** - users choose the best approach for their needs

---

## 🎯 **Motivation: Why Dual-Tree Architecture?**

### **🚨 The Discovery Journey**

**What started as routine testing revealed a critical scalability bottleneck:**

#### **Phase 1: Testing Timestamp Edge Cases**

- Implemented tests for claim receipts at blockchain genesis (slot 0)
- Found timestamp handling works correctly ✅
- Everything seemed fine...

#### **Phase 2: Testing Transaction Size Limits**

- Expected to hit Solana's ~1232 byte transaction limits with large merkle proofs
- **Surprise Discovery**: Large transactions (1400-2200+ bytes) were **accepted**
- Real bottleneck: **Compute exhaustion** (200K compute unit limit), not transaction size

#### **Phase 3: Critical Scalability Limits Discovered**

**Testing revealed exact practical limits:**

```
🧪 Test Results:
• Small proof (5 elements):   ❌ Failed (InvalidMerkleProof - dummy data)
• Medium proof (15 elements): ❌ Failed (compute exhaustion)
• Large proof (25+ elements): ❌ Failed (compute exhaustion)
• ✅ PRACTICAL LIMIT: 10-12 merkle proof elements maximum
```

**Compute Budget Breakdown:**

- Base instruction overhead: ~25K CUs
- Associated Token Account creation: ~22K CUs
- ClaimReceipt PDA creation: ~15K CUs
- **Available for merkle verification: ~138K CUs**

#### **Phase 4: The Scalability Crisis**

**Binary merkle tree math revealed the problem:**

```
🌳 Binary Tree Scalability Reality:
• Tree depth ≈ log₂(claimants per cohort)
• 1,000 claimants   = ~10 levels = 10 proof elements = ✅ Barely viable
• 4,000 claimants   = ~12 levels = 12 proof elements = ⚠️  At the limit
• 10,000 claimants  = ~14 levels = 14 proof elements = ❌ UNUSABLE
• 1,000,000 claimants = ~20 levels = 20 proof elements = ❌ CATASTROPHIC
```

**🔥 The Crisis:**

- **System needs**: Millions of claimants across hundreds of cohorts
- **Current limit**: ~1,000-4,000 claimants per cohort maximum
- **Gap**: 250x to 1,000x scalability shortfall

### **💡 The Solution: 256-ary Merkle Trees**

**256-ary tree math reveals the solution:**

```
🌳 256-ary Tree Scalability Transformation:
• Tree depth ≈ log₂₅₆(claimants per cohort)
• 1,000 claimants     = ~2 levels = 2 proof elements = ✅ Excellent
• 65,000 claimants    = ~3 levels = 3 proof elements = ✅ Great
• 16,000,000 claimants = ~4 levels = 4 proof elements = ✅ Still viable!
```

**🚀 The Transformation:**

- **4,000x improvement** in single-cohort capacity
- **System transforms** from "thousands per cohort" to "millions per cohort"
- **User experience** improves (smaller proofs, faster claims)

### **🎯 Why Dual-Tree (Not Migration)?**

**Instead of breaking changes, we're implementing both side-by-side:**

#### **Benefits of Dual Approach:**

- ✅ **Zero breaking changes** - existing campaigns continue working
- ✅ **Risk mitigation** - can rollback if V1 has issues
- ✅ **Performance comparison** - real-world data on both approaches
- ✅ **Gradual migration** - ecosystem adopts V1 organically
- ✅ **Campaign flexibility** - creators choose appropriate tree type

#### **Use Case Mapping:**

- **Small campaigns** (<1,000 claimants): V0 binary trees work fine
- **Medium campaigns** (1,000-4,000): V0 risky, V1 recommended
- **Large campaigns** (4,000+): V1 required, V0 unusable
- **Massive campaigns** (50,000+): V1 strategic advantage

### **📈 Expected Outcomes**

**After implementation:**

- **Current users**: No disruption, everything works as before
- **New large campaigns**: Can scale to millions of claimants per cohort
- **Performance**: Comprehensive benchmarks comparing both approaches
- **Future-proof**: System ready for massive token distributions

**This dual-tree architecture transforms Prism Protocol from a "medium-scale" to "web-scale" token distribution system!** 🌐

---

## 🎯 **Phase 1: On-Chain Program Changes**

### **Step 1.1: Create Verification Functions**

**File**: `programs/prism-protocol/src/instructions/claim_tokens_v0.rs`

```rust
use crate::merkle_proof_types::{ProofV0, ProofV1}; // Import proof types

// Keep existing function (rename for clarity)
fn verify_merkle_proof_v0(proof: &ProofV0, root: &[u8; 32], leaf: &[u8; 32]) -> bool {
    let mut computed_hash = *leaf;
    for p_elem in proof.as_slice().iter() {
        let mut hasher = SolanaHasher::default();
        hasher.hash(&[0x01]); // Internal node prefix
        if computed_hash <= *p_elem {
            hasher.hash(&computed_hash);
            hasher.hash(p_elem);
        } else {
            hasher.hash(p_elem);
            hasher.hash(&computed_hash);
        }
        computed_hash = hasher.result().to_bytes();
    }
    computed_hash == *root
}

// Add new 256-ary verification function
fn verify_merkle_proof_v1(proof: &ProofV1, root: &[u8; 32], leaf: &[u8; 32]) -> bool {
    let mut computed_hash = *leaf;

    for level_siblings in proof.as_slice().iter() {
        // Collect all sibling hashes + current hash
        let mut all_hashes = level_siblings.clone();
        all_hashes.push(computed_hash);
        all_hashes.sort(); // Deterministic ordering

        // Hash all children together
        let mut hasher = SolanaHasher::default();
        hasher.hash(&[0x01]); // Internal node prefix
        for hash in all_hashes {
            hasher.hash(&hash);
        }
        computed_hash = hasher.result().to_bytes();
    }
    computed_hash == *root
}
```

### **Step 1.2: Create ClaimTokensV1 Instruction**

**File**: `programs/prism-protocol/src/instructions/claim_tokens_v1.rs`

```rust
use anchor_lang::prelude::*;
use crate::merkle_proof_types::ProofV1;
// ... other imports similar to claim_tokens_v0.rs

#[derive(Accounts)]
#[instruction(
    campaign_fingerprint: [u8; 32],
    merkle_root: [u8; 32],
    merkle_proof: ProofV1,  // ← Clean type-safe proof
    assigned_vault_index: u8,
    entitlements: u64
)]
pub struct ClaimTokensV1<'info> {
    // Same account structure as V0
}

pub fn handle_claim_tokens_v1(
    ctx: Context<ClaimTokensV1>,
    _campaign_fingerprint: [u8; 32],
    cohort_merkle_root: [u8; 32],
    merkle_proof: ProofV1, // ← Type-safe 256-ary proof
    assigned_vault_index: u8,
    entitlements: u64,
) -> Result<()> {
    // Same logic as V0, but call verify_merkle_proof_v1
    if !verify_merkle_proof_v1(&merkle_proof, &cohort.merkle_root, &leaf_hash) {
        return err!(ErrorCode::InvalidMerkleProof);
    }
    // ... rest identical to V0
}
```

### **Step 1.3: Update Program Entry Points**

**File**: `programs/prism-protocol/src/lib.rs`

```rust
pub mod instructions {
    pub mod claim_tokens_v0; // Existing
    pub mod claim_tokens_v1; // New
    // ... other instructions
}

#[program]
pub mod prism_protocol {
    // Keep existing
    pub fn claim_tokens_v0(/* ... */) -> Result<()> {
        instructions::claim_tokens_v0::handle_claim_tokens_v0(ctx, /* ... */)
    }

    // Add new
    pub fn claim_tokens_v1(/* ... */) -> Result<()> {
        instructions::claim_tokens_v1::handle_claim_tokens_v1(ctx, /* ... */)
    }
}
```

---

## 🌳 **Phase 2: Merkle Crate Changes**

### **Step 2.0: Create Proof Types (Type Safety)**

**File**: `crates/prism-protocol-merkle/src/proof_types.rs`

```rust
use anchor_lang::prelude::*;

/// Binary tree proof for V0 claim instructions
#[derive(Clone, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct ProofV0(Vec<[u8; 32]>);

impl ProofV0 {
    pub fn new(proof: Vec<[u8; 32]>) -> Self {
        Self(proof)
    }

    pub fn as_slice(&self) -> &[[u8; 32]] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_inner(self) -> Vec<[u8; 32]> {
        self.0
    }
}

/// 256-ary tree proof for V1 claim instructions
#[derive(Clone, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct ProofV1(Vec<Vec<[u8; 32]>>);

impl ProofV1 {
    pub fn new(proof: Vec<Vec<[u8; 32]>>) -> Self {
        Self(proof)
    }

    pub fn as_slice(&self) -> &[Vec<[u8; 32]>] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_inner(self) -> Vec<Vec<[u8; 32]>> {
        self.0
    }

    /// Get total number of hashes across all levels
    pub fn total_hashes(&self) -> usize {
        self.0.iter().map(|level| level.len()).sum()
    }
}

// Implement From traits for easy conversion
impl From<Vec<[u8; 32]>> for ProofV0 {
    fn from(proof: Vec<[u8; 32]>) -> Self {
        Self::new(proof)
    }
}

impl From<Vec<Vec<[u8; 32]>>> for ProofV1 {
    fn from(proof: Vec<Vec<[u8; 32]>>) -> Self {
        Self::new(proof)
    }
}
```

**🎯 Benefits of Proof Types:**

- **Type Safety**: Impossible to pass V0 proof to V1 function (compile-time error)
- **API Clarity**: Function signatures immediately show which tree version they expect
- **Better Errors**: Clear compiler messages when using wrong proof type
- **Extensibility**: Easy to add methods like `total_hashes()` or `depth()`
- **Self-Documenting**: Code becomes more readable and maintainable

### **Step 2.1: Create PrismHasherV1**

**File**: `crates/prism-protocol-merkle/src/hasher_v1.rs`

```rust
use anchor_lang::solana_program::hash::Hasher as SolanaHasher;

/// 256-ary merkle tree hasher
#[derive(Clone, Debug)]
pub struct PrismHasherV1;

impl PrismHasherV1 {
    // Keep leaf hashing identical to V0
    pub fn hash_leaf(data: &[u8]) -> [u8; 32] {
        let mut hasher = SolanaHasher::default();
        hasher.hash(&[0x00]); // Leaf prefix
        hasher.hash(data);
        hasher.result().to_bytes()
    }

    // New: Hash multiple children for internal nodes
    pub fn hash_internal_node(children: &[[u8; 32]]) -> [u8; 32] {
        let mut hasher = SolanaHasher::default();
        hasher.hash(&[0x01]); // Internal node prefix

        // Sort children for deterministic ordering
        let mut sorted_children = children.to_vec();
        sorted_children.sort();

        for child in sorted_children {
            hasher.hash(&child);
        }
        hasher.result().to_bytes()
    }
}
```

### **Step 2.2: Create ClaimTreeV1**

**File**: `crates/prism-protocol-merkle/src/builder_v1.rs`

````rust
use std::collections::HashMap;
use crate::{hash_claim_leaf, ClaimLeaf, PrismHasherV1, ProofV1};

const TREE_WIDTH: usize = 256; // 256-ary tree

pub struct ClaimTreeV1 {
    pub root: Option<[u8; 32]>,
    pub claimant_to_index: HashMap<Pubkey, usize>,
    pub leaves: Vec<ClaimLeaf>,
    // Store tree structure for proof generation
    pub tree_levels: Vec<Vec<[u8; 32]>>,
}

impl ClaimTreeV1 {
    pub fn from_leaves(leaves: Vec<ClaimLeaf>) -> Result<Self> {
        require!(!leaves.is_empty(), ErrorCode::InvalidInput);

        // Create claimant mapping
        let mut claimant_to_index = HashMap::new();
        for (index, leaf) in leaves.iter().enumerate() {
            if claimant_to_index.insert(leaf.claimant, index).is_some() {
                return err!(ErrorCode::DuplicateClaimant);
            }
        }

        // Hash all leaves
        let leaf_hashes: Vec<[u8; 32]> = leaves
            .iter()
            .map(|leaf| hash_claim_leaf(leaf))
            .collect();

        // Build 256-ary tree
        let (root, tree_levels) = Self::build_wide_tree(leaf_hashes);

        Ok(ClaimTreeV1 {
            root,
            claimant_to_index,
            leaves,
            tree_levels,
        })
    }

    fn build_wide_tree(mut current_level: Vec<[u8; 32]>) -> (Option<[u8; 32]>, Vec<Vec<[u8; 32]>>) {
        let mut all_levels = vec![current_level.clone()];

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            // Process in chunks of TREE_WIDTH (256)
            for chunk in current_level.chunks(TREE_WIDTH) {
                let parent_hash = PrismHasherV1::hash_internal_node(chunk);
                next_level.push(parent_hash);
            }

            all_levels.push(next_level.clone());
            current_level = next_level;
        }

        let root = current_level.first().copied();
        (root, all_levels)
    }

    pub fn proof_for_claimant(&self, claimant: &Pubkey) -> Result<ProofV1> {
        let index = self.claimant_to_index
            .get(claimant)
            .ok_or(ErrorCode::ClaimantNotFound)?;

        let mut proof = Vec::new();
        let mut current_index = *index;

        // Generate proof level by level
        for level in 0..(self.tree_levels.len() - 1) {
            let current_level_hashes = &self.tree_levels[level];

            // Find the chunk this index belongs to
            let chunk_start = (current_index / TREE_WIDTH) * TREE_WIDTH;
            let chunk_end = std::cmp::min(chunk_start + TREE_WIDTH, current_level_hashes.len());

            // Collect all siblings in this chunk (excluding the target)
            let mut siblings = Vec::new();
            for i in chunk_start..chunk_end {
                if i != current_index {
                    siblings.push(current_level_hashes[i]);
                }
            }

            proof.push(siblings);
            current_index /= TREE_WIDTH; // Move to parent index
        }

        Ok(ProofV1::new(proof))
    }
}

### **Step 2.3: Update Merkle Crate Exports**

**File**: `crates/prism-protocol-merkle/src/lib.rs`

```rust
pub mod builder;      // Existing V0
pub mod builder_v1;   // New V1
pub mod hasher;       // Existing V0
pub mod hasher_v1;    // New V1
pub mod proof;
pub mod proof_types;  // New proof types

// V0 exports (existing)
pub use builder::{create_merkle_tree, ClaimTree};
pub use hasher::PrismHasher;

// V1 exports (new)
pub use builder_v1::ClaimTreeV1;
pub use hasher_v1::PrismHasherV1;

// Proof type exports (new)
pub use proof_types::{ProofV0, ProofV1};

// Common exports
pub use prism_protocol::{hash_claim_leaf, ClaimLeaf};
````

---

## 🔧 **Phase 3: SDK Changes**

### **Step 3.1: Create Compilation Functions**

**File**: `crates/prism-protocol-sdk/src/campaign_compiler_v1.rs`

```rust
use crate::campaign_compiler::{CompiledCampaign, CompiledCohort}; // Reuse base types
use prism_protocol_merkle::{ClaimTreeV1, hash_claim_leaf, ProofV1};

pub fn compile_campaign_v1(
    campaign_config: &CampaignConfig,
) -> CompilerResult<CompiledCampaignV1> {
    // Similar to existing compile_campaign but use ClaimTreeV1
    let cohorts = compile_cohorts_v1(&campaign_config.cohorts)?;

    Ok(CompiledCampaignV1 {
        // Same fields as V0 but with V1 cohorts
        cohorts,
        // ... other fields
    })
}

fn compile_cohorts_v1(
    cohort_configs: &[CohortConfig],
) -> CompilerResult<Vec<CompiledCohortV1>> {
    cohort_configs
        .iter()
        .map(|config| {
            let leaves = /* ... same leaf generation as V0 ... */;

            // Use V1 tree builder
            let merkle_tree = ClaimTreeV1::from_leaves(leaves)
                .map_err(|_| CompilerError::MerkleTreeBuildFailed)?;

            Ok(CompiledCohortV1 {
                merkle_tree, // ClaimTreeV1 instead of ClaimTree
                // ... other fields same as V0
            })
        })
        .collect()
}

// V1-specific types that wrap V0 types with V1 merkle trees
pub struct CompiledCampaignV1 {
    pub cohorts: Vec<CompiledCohortV1>,
    // ... same other fields as CompiledCampaign
}

pub struct CompiledCohortV1 {
    pub merkle_tree: ClaimTreeV1, // ← Key difference
    // ... same other fields as CompiledCohort
}
```

### **Step 3.2: Create Instruction Builders**

**File**: `crates/prism-protocol-sdk/src/instruction_builders_v1.rs`

```rust
use prism_protocol_merkle::ProofV1;

pub fn build_claim_tokens_v1_ix(
    address_finder: &AddressFinder,
    admin: Pubkey,
    claimant: Pubkey,
    mint: Pubkey,
    claimant_token_account: Pubkey,
    campaign_fingerprint: [u8; 32],
    cohort_merkle_root: [u8; 32],
    merkle_proof: ProofV1, // ← Type-safe V1 proof
    assigned_vault_index: u8,
    entitlements: u64,
) -> SdkResult<(Instruction, Pubkey, Pubkey)> {
    // Same account resolution as V0
    // Different instruction data structure

    let instruction_data = ClaimTokensV1 {
        campaign_fingerprint,
        merkle_root: cohort_merkle_root,
        merkle_proof, // ProofV1 type
        assigned_vault_index,
        entitlements,
    };

    // ... rest similar to V0 but targeting claim_tokens_v1
}
```

### **Step 3.3: Update SDK Exports**

**File**: `crates/prism-protocol-sdk/src/lib.rs`

```rust
// V0 exports (existing)
pub use campaign_compiler::{compile_campaign, CompiledCampaign, CompiledCohort};
pub use instruction_builders::build_claim_tokens_v0_ix;

// V1 exports (new)
pub use campaign_compiler_v1::{compile_campaign_v1, CompiledCampaignV1, CompiledCohortV1};
pub use instruction_builders_v1::build_claim_tokens_v1_ix;

// Re-export proof types for convenience
pub use prism_protocol_merkle::{ProofV0, ProofV1};

// Common exports
pub use campaign_compiler::{CampaignConfig, CohortConfig}; // Config types stay same
```

---

## 🧪 **Phase 4: Testing Strategy**

### **Step 4.1: Keep All Existing Tests**

- ✅ All current tests continue to work unchanged
- ✅ `test_claim_maximum_merkle_proof_size.rs` remains as V0 test

### **Step 4.2: Create Parallel V1 Tests**

**File**: `crates/prism-protocol-testing/tests/test_claim_maximum_merkle_proof_size_v1.rs`

```rust
// Copy structure from V0 test but use:
// - build_claim_tokens_v1_ix
// - Vec<Vec<[u8; 32]>> proof structure
// - Much larger proof sizes (test up to 100+ elements)

#[test]
fn test_claim_maximum_merkle_proof_size_v1() {
    // Test 256-ary tree limits
    // Should handle much larger cohorts than V0
}
```

### **Step 4.3: Create Comparison Tests**

**File**: `crates/prism-protocol-testing/tests/test_merkle_tree_comparison.rs`

```rust
#[test]
fn test_binary_vs_wide_tree_performance() {
    // Same claimant data, build both V0 and V1 trees
    // Compare:
    // - Proof sizes
    // - Compute unit consumption
    // - Tree construction time
    // - Verification performance
}

#[test]
fn test_scalability_crossover_point() {
    // Find the claimant count where V1 becomes better than V0
}
```

---

## 📊 **Phase 5: Validation & Migration**

### **Step 5.1: Performance Benchmarks**

- Test both approaches with various cohort sizes (100, 1K, 10K, 100K, 1M claimants)
- Measure compute units, proof sizes, and user experience
- Document the crossover point where V1 becomes preferable

### **Step 5.2: Ecosystem Integration**

- Update campaign creation tools to support both tree types
- Provide migration tooling for existing campaigns
- Create decision framework for tree type selection

### **Step 5.3: Documentation**

- Update all docs to explain both approaches
- Provide migration guides
- Create performance comparison charts

---

## ✅ **Success Criteria**

- [ ] Both V0 and V1 claim instructions work correctly
- [ ] All existing tests continue to pass
- [ ] V1 supports millions of claimants per cohort
- [ ] Performance benchmarks show expected improvements
- [ ] Zero breaking changes to existing functionality
- [ ] Clear migration path for large campaigns

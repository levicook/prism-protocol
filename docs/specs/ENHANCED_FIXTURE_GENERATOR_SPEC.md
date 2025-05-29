# Enhanced Fixture Generator Specification

## 🎯 Overview

The enhanced fixture generator creates organized, campaign-specific test directories with real keypairs, facilitating comprehensive end-to-end testing of the claiming ecosystem. All generated claimants use real, random Solana keypairs for realistic testing scenarios.

## 📁 Proposed Directory Structure

```
test-artifacts/
├── fixtures/
│   ├── test-campaign-001/           # Test fixture source (slugified campaign name)
│   │   ├── campaign.csv             # Campaign definition (source)
│   │   ├── cohorts.csv             # Cohort configuration (source)
│   │   ├── claimant-keypairs/      # Real random keypairs
│   │   │   ├── claimant-0001.json  # First claimant keypair
│   │   │   ├── claimant-0002.json  # Second claimant keypair
│   │   │   └── ...
│   │   └── README.md               # Test fixture summary and usage
│   └── stress-test-100k/           # Large-scale test fixture
│       ├── campaign.csv
│       ├── cohorts.csv
│       └── claimant-keypairs/
│           └── ... (100,000 files)
└── campaigns/                      # Compiled test campaigns (API-servable)
    ├── test-campaign-001.db        # Compiled from test-artifacts/fixtures/test-campaign-001/
    └── stress-test-100k.db         # Compiled from test-artifacts/fixtures/stress-test-100k/

campaigns/                          # Compiled production campaigns (API-servable)
├── pengu-airdrop-season1.db       # Production compiled campaigns
└── community-rewards-q1.db
```

**Key Concepts:**
- **Campaign** = CSV files (human-readable source)
- **Compiled Campaign** = SQLite database (API-servable, deployable)
- **API Server** reads from a single campaigns directory (test-artifacts/campaigns/ OR campaigns/)

## 🔧 Enhanced CLI Interface

### New Default Behavior

```bash
# Generate test fixture source files
cargo run -p prism-protocol-cli -- generate-fixtures \
  --campaign-name "Test Campaign 001" \    # Creates: test-artifacts/fixtures/test-campaign-001/
  --count 1000 \
  --cohort-count 3 \
  --distribution realistic
```

### Full Parameter Set

```bash
cargo run -p prism-protocol-cli -- generate-fixtures \
  --campaign-name "Pengu Airdrop Season 1" \        # Campaign identifier (slugified for directory)
  --output-dir "test-artifacts/fixtures/" \          # Base output directory (default)
  --count 10000 \                                   # Number of claimants
  --cohort-count 3 \                               # Number of cohorts
  --distribution realistic \                        # Distribution type
  --seed 42                                        # Optional: deterministic generation
```

## 🔑 Simplified Keypair Implementation

### Random Keypair Generation

```rust
use solana_sdk::signature::{Keypair, Signer};
use rand::thread_rng;

// Simple random keypair generation
fn generate_claimant_keypair() -> Keypair {
    Keypair::new()
}

// For deterministic testing (with seed)
fn generate_deterministic_keypair(seed: u64, index: u32) -> Keypair {
    use rand::{SeedableRng, rngs::StdRng};
    let mut rng = StdRng::seed_from_u64(seed.wrapping_add(index as u64));
    Keypair::generate(&mut rng)
}
```

### Keypair File Format

```json
// claimant-0001.json
{
  "keypair": [/* 64-byte array */],
  "pubkey": "7BgBvyjrZX8YKHGoW9Y8929nsq6TsQANzvsGVEpVLUD8",
  "index": 1,
  "campaign": "test-campaign-001",
  "cohort": "early_contributors",
  "entitlements": 5
}
```

## 📊 Realistic Workflow Examples

### Test Fixture → Compiled Campaign Workflow

```bash
# 1. Generate test fixture source files
cargo run -p prism-protocol-cli -- generate-fixtures \
  --campaign-name "Pengu Airdrop Test" \
  --count 1000 \
  --distribution realistic

# 2. Compile test fixture to API-servable database
cd test-artifacts/fixtures/pengu-airdrop-test/
cargo run -p prism-protocol-cli -- compile-campaign \
  --campaign-csv campaign.csv \
  --cohorts-csv cohorts.csv \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --admin-keypair ../../../test-admin.json \
  --campaign-db-out ../../campaigns/pengu-airdrop-test.db

# 3. Test deployment from compiled campaign
cargo run -p prism-protocol-cli -- deploy-campaign \
  --campaign-db-in ../../campaigns/pengu-airdrop-test.db \
  --admin-keypair ../../../test-admin.json

# 4. Serve API from compiled campaigns directory
cargo run -p prism-protocol-cli -- serve-api \
  --campaigns-dir ../../campaigns/
```

### Production Campaign Workflow

```bash
# 1. Create production campaign source from tested fixture
mkdir -p campaigns-source/pengu-airdrop-season1/
cp test-artifacts/fixtures/pengu-airdrop-test/{campaign.csv,cohorts.csv} \
   campaigns-source/pengu-airdrop-season1/

# 2. Compile production campaign directly to campaigns directory
cd campaigns-source/pengu-airdrop-season1/
cargo run -p prism-protocol-cli -- compile-campaign \
  --campaign-csv campaign.csv \
  --cohorts-csv cohorts.csv \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --admin-keypair ../../secrets/mainnet-admin.json \
  --campaign-db-out ../../campaigns/pengu-airdrop-season1.db

# 3. Deploy to mainnet from compiled campaign
cargo run -p prism-protocol-cli -- deploy-campaign \
  --campaign-db-in ../../campaigns/pengu-airdrop-season1.db \
  --admin-keypair ../../secrets/mainnet-admin.json \
  --rpc-url https://api.mainnet-beta.solana.com

# 4. Serve API from production campaigns directory
cargo run -p prism-protocol-cli -- serve-api \
  --campaigns-dir ../../campaigns/ \
  --port 3000
```

## 📊 Campaign Metadata

### README.md Generation

```markdown
# Test Campaign: pengu-airdrop-test

Generated: 2024-01-15 10:30:00 UTC
Claimants: 1,000 (with real keypairs)
Cohorts: 3
Distribution: realistic

## Testing Workflow

This fixture provides a complete testing environment for the Pengu Airdrop campaign.

### Quick Test
```bash
# Compile and deploy test campaign
cargo run -p prism-protocol-cli -- compile-campaign \
  --campaign-csv campaign.csv \
  --cohorts-csv cohorts.csv \
  --mint YOUR_TEST_MINT \
  --admin-keypair ../../../test-admin.json \
  --campaign-db-out campaign.db

# Test claiming with first claimant
cargo run -p prism-protocol-cli -- claim-tokens \
  --campaign-db campaign.db \
  --claimant-keypair claimant-keypairs/claimant-0001.json
```

### Production Deployment
```bash
# Copy to campaigns folder for production
mkdir -p campaigns/pengu-airdrop-season1/
cp {campaign.csv,cohorts.csv} campaigns/pengu-airdrop-season1/
cd campaigns/pengu-airdrop-season1/
# ... compile with production settings
```
```

## 🧪 Testing Integration

### Updated Test Scripts

```bash
# scripts/test-e2e-enhanced.sh
CAMPAIGN_NAME="pengu-airdrop-e2e-$(date +%s)"
FIXTURE_DIR="test-artifacts/fixtures/$(echo "$CAMPAIGN_NAME" | sed 's/[^a-zA-Z0-9]/-/g')"
CAMPAIGN_DB="test-artifacts/campaigns/$(echo "$CAMPAIGN_NAME" | sed 's/[^a-zA-Z0-9]/-/g').db"

# Generate test fixture source (always with real keypairs)
cargo run -p prism-protocol-cli -- generate-fixtures \
  --campaign-name "$CAMPAIGN_NAME" \
  --count 50

# Compile to test campaigns directory
cd "$FIXTURE_DIR"
cargo run -p prism-protocol-cli -- compile-campaign \
  --campaign-csv campaign.csv \
  --cohorts-csv cohorts.csv \
  --mint "$MINT_PUBKEY" \
  --admin-keypair ../../test-admin.json \
  --campaign-db-out "../../$CAMPAIGN_DB"

# Test claiming with real claimants
for keypair in claimant-keypairs/claimant-*.json; do
  cargo run -p prism-protocol-cli -- claim-tokens \
    --campaign-db "../../$CAMPAIGN_DB" \
    --claimant-keypair "$keypair" &
done
wait

# Test API server on compiled campaigns
cargo run -p prism-protocol-cli -- serve-api \
  --campaigns-dir ../../campaigns/ \
  --port 3001 &
API_PID=$!
# ... test API endpoints ...
kill $API_PID
```

## 🎯 Implementation Checklist

### Core Functionality
- [ ] Remove `--real-keypairs` flag (always generate real keypairs)
- [ ] Implement simple random keypair generation
- [ ] Create organized directory structure with slugified names
- [ ] Generate and save individual keypair files
- [ ] Create campaign README.md with realistic workflow examples

### Integration
- [ ] Update examples to show fixtures → campaigns workflow
- [ ] Use evocative campaign names in documentation
- [ ] Guide users toward API-servable database organization
- [ ] Update test scripts to use new simplified interface

### Documentation
- [ ] Update CLI help text to reflect simplified approach
- [ ] Provide clear examples for both testing and production workflows
- [ ] Show how to transition from fixtures to production campaigns

## 💡 Future Enhancements

### Campaign Templates
```bash
# Predefined campaign templates with realistic names
cargo run -p prism-protocol-cli -- generate-fixtures \
  --template "airdrop-standard" \
  --campaign-name "my-token-genesis-drop"
```

### Integration with Campaign Admin dApp
```bash
# Future: Generate fixtures from admin dApp configuration
cargo run -p prism-protocol-cli -- generate-fixtures \
  --from-admin-config campaign-admin-export.json \
  --campaign-name "designed-in-admin-ui"
``` 
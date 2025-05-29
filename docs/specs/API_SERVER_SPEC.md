# Prism Protocol API Server Specification

## 🎯 Overview

The API server provides HTTP endpoints for claimants to retrieve merkle proofs and campaign information. It operates solely from compiled campaign databases, making it portable and stateless. The API server is implemented as a subcommand of the main CLI: `prism-protocol-cli serve-api`.

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   dApp/CLI      │───▶│ CLI serve-api   │───▶│  Campaign DB    │
│                 │    │                 │    │   (SQLite)      │
│ - Wallet conn   │    │ - Proof serving │    │ - Merkle proofs │
│ - TX signing    │    │ - TX building   │    │ - Claimant data │
│ - UI/UX         │    │ - Validation    │    │ - Campaign meta │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## 📡 API Endpoints

### Core Proof Serving

```http
GET /api/campaigns/{fingerprint}/claimants/{pubkey}/proofs
```
**Response:**
```json
{
  "claimant": "7BgBvyjrZX8YKHGoW9Y8929nsq6TsQANzvsGVEpVLUD8",
  "campaign_fingerprint": "abc123...",
  "eligible_cohorts": [
    {
      "cohort_name": "early_contributors",
      "merkle_root": "def456...",
      "entitlements": 5,
      "assigned_vault_index": 1,
      "assigned_vault_pubkey": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
      "merkle_proof": ["hash1", "hash2", "hash3"],
      "amount_per_entitlement": 1000000000,
      "total_claimable": 5000000000
    }
  ],
  "total_claimable_across_cohorts": 5000000000
}
```

### Campaign Information

```http
GET /api/campaigns/{fingerprint}/status
```
**Response:**
```json
{
  "campaign_fingerprint": "abc123...",
  "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "admin": "AdminPubkey...",
  "is_active": true,
  "total_claimants": 1000,
  "total_cohorts": 3,
  "deployment": {
    "deployed_at": "2024-01-15T10:30:00Z",
    "campaign_signature": "sig123...",
    "rpc_url": "https://api.mainnet-beta.solana.com"
  }
}
```

```http
GET /api/campaigns/{fingerprint}/cohorts
```
**Response:**
```json
{
  "cohorts": [
    {
      "name": "early_contributors",
      "merkle_root": "def456...",
      "claimant_count": 334,
      "amount_per_entitlement": 1000000000,
      "vault_count": 2,
      "total_tokens_required": 334000000000
    }
  ]
}
```

### Transaction Building (Optional)

```http
POST /api/campaigns/{fingerprint}/claimants/{pubkey}/build-claim-tx
```
**Request:**
```json
{
  "cohort_name": "early_contributors",
  "rpc_url": "https://api.mainnet-beta.solana.com"
}
```
**Response:**
```json
{
  "transaction": "base64-encoded-transaction",
  "instructions": [
    {
      "program_id": "PrismProgramId...",
      "accounts": [...],
      "data": "base64-encoded-data"
    }
  ],
  "signers_required": ["claimant_pubkey"],
  "estimated_compute_units": 45000,
  "estimated_fee_lamports": 5000
}
```

### Jito Bundle Building (Advanced)

```http
POST /api/campaigns/{fingerprint}/claimants/{pubkey}/build-jito-bundle
```
**Request:**
```json
{
  "cohorts": ["early_contributors", "community_mvps"],
  "tip_lamports": 10000,
  "rpc_url": "https://api.mainnet-beta.solana.com"
}
```
**Response:**
```json
{
  "bundle": {
    "transactions": ["tx1_base64", "tx2_base64"],
    "tip_transaction": "tip_tx_base64"
  },
  "total_compute_units": 90000,
  "estimated_total_fee": 15000
}
```

## 🔧 Implementation Details

### Technology Stack

```toml
# Added to existing prism-protocol-cli Cargo.toml
[dependencies]
# Existing CLI dependencies...
axum = "0.7"           # HTTP server framework (optional feature)
tokio = "1.0"          # Async runtime (optional feature)
rusqlite = "0.36"      # SQLite database access (existing)
serde = "1.0"          # JSON serialization (existing)
tower = "0.4"          # Middleware (optional feature)
tower-http = "0.5"     # HTTP middleware (CORS, logging) (optional feature)
# solana-sdk already included
# hex already included
```

### Feature Flags

```toml
[features]
default = ["cli"]
cli = []
api-server = ["axum", "tokio/rt-multi-thread", "tower", "tower-http"]
```

### Database Schema Integration

The API server reads from the existing campaign database schema:

```sql
-- Existing tables used by API server
SELECT * FROM campaign WHERE fingerprint = ?;
SELECT * FROM cohorts WHERE campaign_fingerprint = ?;
SELECT * FROM claimants WHERE claimant_pubkey = ? AND campaign_fingerprint = ?;
SELECT * FROM vaults WHERE campaign_fingerprint = ? AND cohort_name = ?;
```

### Error Handling

```json
// Standard error response format
{
  "error": {
    "code": "CLAIMANT_NOT_FOUND",
    "message": "Claimant not found in any cohort for this campaign",
    "details": {
      "claimant": "7BgBvyjrZX8YKHGoW9Y8929nsq6TsQANzvsGVEpVLUD8",
      "campaign_fingerprint": "abc123..."
    }
  }
}
```

**Error Codes:**
- `CAMPAIGN_NOT_FOUND` - Campaign fingerprint not found
- `CLAIMANT_NOT_FOUND` - Claimant not eligible for any cohort
- `ALREADY_CLAIMED` - Claimant has already claimed (if tracking enabled)
- `CAMPAIGN_INACTIVE` - Campaign is not active for claiming
- `INVALID_COHORT` - Requested cohort doesn't exist
- `DATABASE_ERROR` - Internal database error

## 🚀 Deployment & Configuration

### CLI Interface

```bash
# Serve test campaigns
cargo run -p prism-protocol-cli -- serve-api \
  --campaigns-dir test-artifacts/campaigns/ \
  --port 3000 \
  --host 0.0.0.0

# Serve production campaigns
cargo run -p prism-protocol-cli -- serve-api \
  --campaigns-dir campaigns/ \
  --port 3000 \
  --host 0.0.0.0

# Single campaign mode (for development)
cargo run -p prism-protocol-cli -- serve-api \
  --campaign-db campaigns/pengu-airdrop-season1.db \
  --port 3000

# Production configuration
cargo run -p prism-protocol-cli -- serve-api \
  --config api-config.yaml \
  --log-level info
```

### Configuration File

```yaml
# api-config.yaml
server:
  host: "0.0.0.0"
  port: 3000
  
campaigns:
  # Single campaign mode
  # database: "campaigns/my-campaign.db"
  
  # Multi-campaign mode
  directory: "campaigns/"
  # For testing: directory: "test-artifacts/campaigns/"

features:
  transaction_building: true
  jito_bundles: true
  claim_status_tracking: false

security:
  cors_origins: ["https://my-dapp.com"]
  rate_limit:
    requests_per_minute: 100
    burst_size: 10

logging:
  level: "info"
  format: "json"

rpc:
  default_url: "https://api.mainnet-beta.solana.com"
  timeout_seconds: 30
```

### Docker Support

```dockerfile
# Dockerfile for unified CLI (includes API server)
FROM rust:1.82 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p prism-protocol-cli --features api-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/prism-protocol-cli /usr/local/bin/
EXPOSE 3000
# Can run any CLI command including serve-api
CMD ["prism-protocol-cli", "serve-api", "--config", "/config/api-config.yaml"]
```

```yaml
# docker-compose.yml
services:
  prism-api-test:
    build: .
    ports:
      - "3001:3000"
    volumes:
      - ./test-artifacts/campaigns:/campaigns:ro
      - ./api-config.yaml:/config/api-config.yaml:ro
    environment:
      - RUST_LOG=info
    command: ["prism-protocol-cli", "serve-api", "--campaigns-dir", "/campaigns", "--port", "3000"]
  
  prism-api-prod:
    build: .
    ports:
      - "3000:3000"
    volumes:
      - ./campaigns:/campaigns:ro
      - ./api-config.yaml:/config/api-config.yaml:ro
    environment:
      - RUST_LOG=info
    command: ["prism-protocol-cli", "serve-api", "--campaigns-dir", "/campaigns", "--port", "3000"]
  
  # Could also run other CLI commands in separate containers
  prism-deploy:
    build: .
    volumes:
      - ./campaigns:/campaigns
      - ./secrets:/secrets:ro
    command: ["prism-protocol-cli", "deploy-campaign", "--campaign-db-in", "/campaigns/my-campaign.db"]
```

## 🧪 Testing Strategy

### Unit Tests

```rust
// In apps/prism-protocol-cli/src/commands/serve_api.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_get_claimant_proofs() {
        let app = create_test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/campaigns/abc123/claimants/def456/proofs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        // Verify response structure...
    }
}
```

### Integration Tests

```bash
# scripts/test-api-integration.sh
# Start API server with test campaigns
cargo run -p prism-protocol-cli --features api-server -- serve-api \
  --campaigns-dir test-artifacts/campaigns/ \
  --port 3001 &
API_PID=$!

# Test endpoints
curl -s http://localhost:3001/api/campaigns/abc123/status | jq .
curl -s http://localhost:3001/api/campaigns/abc123/claimants/def456/proofs | jq .

# Cleanup
kill $API_PID
```

### Load Testing

```bash
# Test with many concurrent requests
ab -n 1000 -c 10 http://localhost:3000/api/campaigns/abc123/status
```

## 🔒 Security Considerations

### Rate Limiting
- Per-IP rate limiting to prevent abuse
- Different limits for different endpoint types
- Burst allowance for legitimate usage spikes

### Input Validation
- Validate all pubkeys and fingerprints
- Sanitize database queries to prevent injection
- Validate JSON payloads for transaction building

### CORS Configuration
- Configurable allowed origins
- Proper preflight handling
- Secure defaults for production

### Monitoring
- Request logging with structured format
- Error rate monitoring
- Performance metrics (response times, database query times)

## 📈 Performance Optimization

### Database Optimization
- Proper indexing on frequently queried columns
- Connection pooling for concurrent requests
- Read-only database access (no writes needed)

### Caching Strategy
- In-memory caching of campaign metadata
- Proof caching with TTL
- HTTP caching headers for static data

### Horizontal Scaling
- Stateless design allows multiple instances
- Load balancer configuration
- Database read replicas for high traffic

## 🎯 Implementation Phases

### Phase 1: Core Proof Serving
- [ ] Add `serve-api` subcommand to existing CLI structure
- [ ] Basic HTTP server with campaign database loading (behind feature flag)
- [ ] Claimant proof lookup endpoint
- [ ] Campaign status endpoint
- [ ] Basic error handling

### Phase 2: Transaction Building
- [ ] Claim transaction building endpoint
- [ ] Integration with existing prism-protocol-sdk usage
- [ ] RPC client for on-chain data
- [ ] Comprehensive testing

### Phase 3: Production Features
- [ ] Rate limiting and security
- [ ] Multi-campaign support
- [ ] Monitoring and logging
- [ ] Docker containerization with unified CLI

### Phase 4: Advanced Features
- [ ] Jito bundle building
- [ ] Claim status tracking
- [ ] Performance optimization
- [ ] Horizontal scaling support

## 🔗 CLI Integration Benefits

### Unified Toolchain
- Single binary for all Prism Protocol operations
- Consistent configuration and logging
- Shared database access patterns and utilities

### Simplified Deployment
- One Docker image for all functionality
- Can run different commands in different containers
- Shared secrets and configuration management

### Development Efficiency
- Shared code between CLI commands and API server
- Consistent error handling and validation
- Single build process and test suite 
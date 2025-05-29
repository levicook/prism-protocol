# Testing Without Tears: Docker-First CLI Strategy

## 🎯 The Challenge

Our current testing infrastructure suffers from classic toolchain conflicts that make reliable testing a painful experience:

### **Current Pain Points**

- **🔥 Toolchain Wars**: Standard Rust vs Solana BPF fighting over the `target/` directory
- **⚙️ Complex Setup**: Multiple tool versions (anchor, solana-cli, node) with environment drift
- **🖥️ "Works on My Machine"**: Inconsistent environments between developers and CI/CD
- **⏱️ Time Sink**: More time fighting infrastructure than building features
- **🧪 Brittle Tests**: Test infrastructure more likely to break than the code being tested

### **The Root Problem**

We're at the intersection of **two different Rust toolchains**:

- **Standard Rust**: For CLI tools, SDK, testing utilities
- **Solana BPF**: For on-chain programs with custom target configurations

These toolchains want to own the same `target/` directory, leading to constant compilation conflicts and environment management headaches.

## 🐳 The Docker Solution

**Core Insight**: Let Docker isolate the CLI toolchain while allowing anchor/solana to own the host environment.

### **Strategic Approach**

1. **Containerize the CLI**: `prism-protocol-cli` runs in isolated Docker container
2. **Host Owns Anchor**: Let anchor and solana-test-validator manage the host `target/` directory
3. **Volume Mapping**: Share test artifacts between container and host
4. **Simple Commands**: `docker run prism-protocol-cli generate-fixtures --campaign-name "Test"`
5. **Fast Builds**: Use cargo-chef and multi-stage builds for efficient Docker layers

## 📋 Implementation Strategy

### **Container Scope: CLI Only**

We'll containerize **only the CLI** for these reasons:

- ✅ **Focused Solution**: Solves the specific toolchain conflict
- ✅ **Simple Networking**: No complex container-to-container communication
- ✅ **Host Integration**: Easy file sharing via volumes
- ✅ **Incremental Adoption**: Can add other components later

### **Volume Strategy: test-artifacts**

```bash
# Container reads/writes to shared volume
docker run -v $(pwd)/test-artifacts:/workspace/test-artifacts prism-protocol-cli \
  generate-fixtures --campaign-name "Docker Test Campaign" --count 100
```

**Benefits**:

- Host can inspect generated fixtures
- Compiled campaigns persist between runs
- Easy integration with host-based anchor testing
- Natural separation of test data

### **Networking: Host Mode**

```bash
# Use host networking for simplicity
docker run --network host prism-protocol-cli \
  deploy-campaign --campaign-db-in campaigns/test.db --rpc-url http://localhost:8899
```

**Rationale**:

- No need for isolated networks (yet)
- Direct access to host-based solana-test-validator
- Simplifies development and debugging
- Can evolve to custom networks later if needed

## 🏗️ Fast Docker Build Strategy

Based on proven patterns, we'll implement efficient Docker builds using cargo-chef and multi-stage builds.

### **Multi-Stage Architecture**

```dockerfile
# Stage 1: Dependency planning with cargo-chef
FROM rust:1.86 AS chef
RUN cargo install cargo-chef
WORKDIR /build

# Stage 2: Generate dependency recipe
FROM chef AS planner
COPY Cargo.lock Cargo.toml ./
COPY apps/prism-protocol-cli/Cargo.toml ./apps/prism-protocol-cli/
COPY crates/*/Cargo.toml ./crates/*/
# Generate recipe.json for dependency caching
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build dependencies (heavily cached)
FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 4: Build application (only recompiles on source changes)
COPY . .
RUN cargo build --release --bin prism-protocol

# Stage 5: Minimal runtime image
FROM ubuntu:latest AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/cache/apt/*
COPY --from=builder /build/target/release/prism-protocol /usr/local/bin/
ENTRYPOINT ["prism-protocol"]
```

### **Build Performance Benefits**

- **🚀 Dependency Caching**: Dependencies only rebuild when Cargo.toml changes
- **⚡ Source Rebuilds**: Only application code recompiles on source changes
- **🪶 Minimal Runtime**: Small final image with only the binary
- **🔄 Incremental Builds**: Fast iteration during development

## 🎮 Usage Patterns

### **Development Workflow**

```bash
# Build the container (once or when dependencies change)
make docker-build-prism-cli

# Generate test fixtures
docker run -v $(pwd)/test-artifacts:/workspace/test-artifacts prism-protocol-cli \
  generate-fixtures --campaign-name "My Test Campaign" --count 1000

# Compile campaign (container writes to shared volume)
docker run -v $(pwd)/test-artifacts:/workspace/test-artifacts prism-protocol-cli \
  compile-campaign \
  --campaign-csv-in test-artifacts/fixtures/my-test-campaign/campaign.csv \
  --cohorts-csv-in test-artifacts/fixtures/my-test-campaign/cohorts.csv \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --admin-keypair test-artifacts/test-admin.json \
  --campaign-db-out test-artifacts/campaigns/my-test-campaign.db

# Deploy using host-based solana-test-validator
anchor test  # Runs on host, uses host target/
docker run --network host -v $(pwd)/test-artifacts:/workspace/test-artifacts prism-protocol-cli \
  deploy-campaign \
  --campaign-db-in test-artifacts/campaigns/my-test-campaign.db \
  --admin-keypair test-artifacts/test-admin.json
```

### **Testing Integration**

```bash
# Clean test script using containers
#!/bin/bash
set -e

# Build CLI container
make docker-build-prism-cli

# Generate test fixtures in container
docker run -v $(pwd)/test-artifacts:/workspace/test-artifacts prism-protocol-cli \
  generate-fixtures --campaign-name "CI Test $(date +%s)" --count 50

# Host-based anchor testing
anchor test

# Container-based deployment testing
docker run --network host -v $(pwd)/test-artifacts:/workspace/test-artifacts prism-protocol-cli \
  deploy-campaign --campaign-db-in test-artifacts/campaigns/ci-test-*.db --admin-keypair test-artifacts/test-admin.json

echo "✅ All tests passed!"
```

## 📁 Project Structure

```
prism-protocol/
├── Dockerfile                           # CLI container definition
├── Makefile                            # Build automation
├── test-artifacts/                     # Shared volume mount point
│   ├── fixtures/                      # Enhanced fixture generator output
│   ├── campaigns/                     # Compiled campaign databases
│   └── test-admin.json               # Test keypair
├── infra/
│   └── docker/
│       └── prism-protocol-cli.dockerfile  # Dedicated CLI Dockerfile
└── scripts/
    ├── test-docker.sh                # Docker-based test runner
    └── docker-build.sh              # Container build script
```

## 🎯 Implementation Checklist

### **Phase 1: Basic Containerization**

- [ ] Create multi-stage Dockerfile with cargo-chef
- [ ] Add Makefile with docker build targets
- [ ] Test basic CLI commands in container
- [ ] Verify volume mounting for test-artifacts
- [ ] Validate host networking for RPC connections

### **Phase 2: Testing Integration**

- [ ] Create docker-based test runner script
- [ ] Update CI/CD to use containerized CLI
- [ ] Validate enhanced fixture generator works in container
- [ ] Test compilation and deployment workflows
- [ ] Document developer onboarding with Docker

### **Phase 3: Developer Experience**

- [ ] Add shell aliases for common docker commands
- [ ] Create development documentation
- [ ] Optimize build times and image sizes
- [ ] Add health checks and debugging tools

## 🚀 Expected Benefits

### **Immediate Wins**

- ✅ **No More Toolchain Conflicts**: CLI runs in isolation
- ✅ **Consistent Environments**: Same container everywhere
- ✅ **Simple Setup**: `make docker-build-prism-cli && docker run ...`
- ✅ **CI/CD Ready**: Same container in development and production

### **Long-term Advantages**

- ✅ **Reliable Testing**: Infrastructure as code, version controlled
- ✅ **Team Onboarding**: New developers get working environment instantly
- ✅ **Performance**: Fast builds with proper caching
- ✅ **Scalability**: Foundation for full containerized ecosystem

## 🎯 Success Metrics

- **🎯 Developer Onboarding**: New team member can run tests in < 5 minutes
- **🎯 CI/CD Reliability**: 99%+ test success rate (no environment issues)
- **🎯 Build Performance**: < 2 minutes for CLI container rebuild
- **🎯 Zero Toolchain Conflicts**: No more target/ directory wars

## 📚 Reference Implementation

This strategy draws from proven patterns in production Rust applications:

- **Multi-stage builds** for optimal caching and minimal runtime images
- **cargo-chef** for dependency layer optimization
- **Volume mounting** for seamless host integration
- **Host networking** for development simplicity

The goal is **testing without tears** - reliable, fast, and friction-free testing infrastructure that accelerates development rather than hindering it.

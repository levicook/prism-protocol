FROM rust:1.86 AS chef
RUN cargo install cargo-chef

## Planner Stage (beware, there are many caching optimizations here)
## -------------
FROM chef AS planner
WORKDIR /build
COPY Cargo.lock Cargo.toml ./

## Copy programs first because they change least often
## have to copy entire programs because anchor libs are built different
COPY programs/prism-protocol/ ./programs/prism-protocol/

## Copy crates next because they change less often
COPY crates/prism-protocol-client/Cargo.toml ./crates/prism-protocol-client/
COPY crates/prism-protocol-csvs/Cargo.toml ./crates/prism-protocol-csvs/
COPY crates/prism-protocol-db/Cargo.toml ./crates/prism-protocol-db/
COPY crates/prism-protocol-merkle/Cargo.toml ./crates/prism-protocol-merkle/
COPY crates/prism-protocol-sdk/Cargo.toml ./crates/prism-protocol-sdk/
COPY crates/prism-protocol-testing/Cargo.toml ./crates/prism-protocol-testing/

## Copy apps last because they change most often
COPY apps/prism-protocol-cli/Cargo.toml ./apps/prism-protocol-cli/

# Generate the recipe file based on manifests
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef prepare --recipe-path recipe.json

## Builder Stage
## ------------
FROM chef AS builder
WORKDIR /build

# Copy the recipe from the planner stage
COPY --from=planner /build/recipe.json recipe.json

# Compile dependencies using the recipe.
# This layer is cached as long as recipe.json remains unchanged.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json --target x86_64-unknown-linux-gnu

# Copy the actual source code.
# If only app/crate source code changes, the 'cook' layer above remains cached.
COPY Cargo.lock Cargo.toml ./
COPY apps/ ./apps/
COPY crates/ ./crates/
COPY programs/ ./programs/

# Build the CLI application, using the cached dependencies.
# This step only recompiles the changed application code.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --bin prism-protocol --target x86_64-unknown-linux-gnu

## Runtime Stage
## ------------
FROM ubuntu:latest AS runtime
RUN apt-get update && apt-get install -y ca-certificates libssl3 libc6 libsqlite3-0 && rm -rf /var/cache/apt/*

RUN groupadd -r prism && useradd -r -g prism prism

# Create workspace directory for volume mounting
RUN mkdir -p /workspace/test-artifacts && chown -R prism:prism /workspace

COPY --from=builder /build/target/x86_64-unknown-linux-gnu/release/prism-protocol /usr/local/bin/prism-protocol

USER prism
WORKDIR /workspace
ENTRYPOINT ["prism-protocol"] 
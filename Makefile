# Prism Protocol - Test Automation
.PHONY: test test-unit test-cli test-anchor test-integration test-performance test-e2e test-all clean-test-artifacts help smoke-test dev-test

# Default target
help:
	@echo "Prism Protocol Test Automation"
	@echo ""
	@echo "Available targets:"
	@echo "  test-unit        - Run all unit tests"
	@echo "  test-cli         - Run CLI integration tests"
	@echo "  test-anchor      - Run Anchor on-chain program tests"
	@echo "  test-integration - Run full integration tests"
	@echo "  test-performance - Run performance benchmarks"
	@echo "  test-e2e         - Run end-to-end tests with real network"
	@echo "  test-all         - Run all tests (unit + CLI + anchor + integration + e2e)"
	@echo "  test             - Alias for test-all"
	@echo "  smoke-test       - Quick smoke test for development"
	@echo "  dev-test         - Development test cycle (clean + CLI tests)"
	@echo "  clean-test       - Clean up test artifacts"
	@echo "  help             - Show this help message"

# Run all tests
test: test-all

test-all: test-unit test-cli test-anchor test-integration test-e2e
	@echo "✅ All tests completed successfully!"

# Unit tests for all crates
test-unit:
	@echo "🧪 Running unit tests..."
	@cargo test --workspace --lib
	@echo "✅ Unit tests passed!"

# CLI integration tests
test-cli:
	@echo "🚀 Running CLI integration tests..."
	@./scripts/test-cli.sh
	@echo "✅ CLI tests passed!"

# Anchor on-chain program tests
test-anchor:
	@echo "⚓ Running Anchor on-chain program tests..."
	@cd programs/prism-protocol && anchor test --skip-local-validator
	@echo "✅ Anchor tests passed!"

# Full integration tests (including on-chain simulation)
test-integration:
	@echo "🔗 Running integration tests..."
	@cargo test --workspace --test '*'
	@echo "✅ Integration tests passed!"

# Performance benchmarks
test-performance:
	@echo "⚡ Running performance tests..."
	@./scripts/test-performance.sh
	@echo "✅ Performance tests completed!"

# End-to-end tests with real network
test-e2e:
	@echo "🌐 Running end-to-end tests..."
	@./scripts/test-e2e.sh
	@echo "✅ End-to-end tests passed!"

# Clean up test artifacts
clean-test:
	@echo "🧹 Cleaning test artifacts..."
	@rm -f *.csv *.db test-*.csv test-*.db
	@rm -rf test-artifacts/
	@rm -rf target/test-*
	@rm -rf programs/prism-protocol/.anchor programs/prism-protocol/target
	@echo "✅ Test artifacts cleaned!"

# Development helpers
dev-test: clean-test test-cli
	@echo "🔄 Development test cycle completed!"

# Quick smoke test
smoke-test:
	@echo "💨 Running smoke tests..."
	@cargo check --workspace
	@./scripts/test-cli.sh --quick
	@echo "✅ Smoke tests passed!"

# Continuous integration target
ci-test: test-unit test-cli test-anchor test-integration test-e2e
	@echo "🤖 CI test suite completed!"

# Watch mode for development (requires cargo-watch)
test-watch:
	@echo "👀 Starting test watch mode..."
	@cargo watch -x "test --workspace --lib" -s "make smoke-test"

# Docker build settings
DOCKER_BUILDKIT ?= 1
DOCKER_BUILD_PLATFORM ?= linux/x86_64
DOCKER_BUILD_PROGRESS ?= auto

# Docker image names
PRISM_CLI_IMAGE ?= prism-protocol-cli

# Dockerfile paths
PRISM_CLI_DOCKERFILE ?= infra/docker/prism-protocol-cli.dockerfile

.PHONY: docker-build-prism-cli docker-test-volumes docker-clean

# Build prism-protocol CLI Docker image
docker-build-prism-cli:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build \
		--platform $(DOCKER_BUILD_PLATFORM) \
		--progress=$(DOCKER_BUILD_PROGRESS) \
		-t $(PRISM_CLI_IMAGE) \
		-f $(PRISM_CLI_DOCKERFILE) \
		.

# Test the Docker container with volume mounting
docker-test-volumes: docker-build-prism-cli
	@echo "🧪 Testing Docker CLI with volume mounting..."
	@mkdir -p test-artifacts/fixtures test-artifacts/campaigns
	docker run --rm \
		-v $(PWD)/test-artifacts:/workspace/test-artifacts \
		$(PRISM_CLI_IMAGE) \
		--version
	@echo "✅ Docker CLI container working!"

# Clean up Docker images and containers
docker-clean:
	@echo "🧹 Cleaning up Docker resources..."
	-docker rmi $(PRISM_CLI_IMAGE)
	-docker system prune -f

# Show available commands
docker-help:
	@echo "🐳 Prism Protocol Docker Commands"
	@echo "=================================="
	@echo ""
	@echo "Build Commands:"
	@echo "  make docker-build-prism-cli      - Build prism-protocol CLI Docker image"
	@echo ""
	@echo "Test Commands:"
	@echo "  make docker-test-volumes         - Test CLI container with volume mounting"
	@echo ""
	@echo "Utility Commands:"
	@echo "  make docker-clean               - Clean up Docker images and containers"
	@echo "  make docker-help                - Show this help message"
	@echo ""
	@echo "Variables:"
	@echo "  PRISM_CLI_IMAGE                 - CLI image name (default: prism-protocol-cli)"
	@echo "  PRISM_CLI_DOCKERFILE            - Path to CLI Dockerfile (default: infra/docker/prism-protocol-cli.dockerfile)"
	@echo ""
	@echo "Example Usage:"
	@echo "  # Build and test"
	@echo "  make docker-build-prism-cli"
	@echo "  make docker-test-volumes"
	@echo ""
	@echo "  # Generate fixtures with Docker"
	@echo "  docker run -v \$$(pwd)/test-artifacts:/workspace/test-artifacts prism-protocol-cli \\"
	@echo "    generate-fixtures --campaign-name \"Docker Test\" --count 100" 
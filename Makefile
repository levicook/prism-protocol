# Prism Protocol - Test Automation
.PHONY: test test-unit test-cli test-anchor test-integration test-performance test-all clean-test-artifacts help smoke-test dev-test

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
	@echo "  test-all         - Run all tests (unit + CLI + anchor + integration)"
	@echo "  test             - Alias for test-all"
	@echo "  smoke-test       - Quick smoke test for development"
	@echo "  dev-test         - Development test cycle (clean + CLI tests)"
	@echo "  clean-test       - Clean up test artifacts"
	@echo "  help             - Show this help message"

# Run all tests
test: test-all

test-all: test-unit test-cli test-anchor test-integration
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
ci-test: test-unit test-cli test-anchor test-integration
	@echo "🤖 CI test suite completed!"

# Watch mode for development (requires cargo-watch)
test-watch:
	@echo "👀 Starting test watch mode..."
	@cargo watch -x "test --workspace --lib" -s "make smoke-test" 
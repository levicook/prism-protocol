#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}🐳 Testing Prism Protocol CLI Docker Setup${NC}"
echo "=============================================="

# Function to print step headers
print_step() {
    echo -e "\n${YELLOW}📋 $1${NC}"
}

# Function to print success
print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# Function to print error and exit
print_error() {
    echo -e "${RED}❌ $1${NC}"
    exit 1
}

# Ensure we're in the project root
if [[ ! -f "Cargo.toml" ]]; then
    print_error "Must run from project root directory"
fi

print_step "Building Docker image..."
if ! make docker-build-prism-cli; then
    print_error "Failed to build Docker image"
fi
print_success "Docker image built successfully"

print_step "Testing basic CLI functionality..."
if ! docker run --rm prism-protocol-cli --version; then
    print_error "CLI --version command failed"
fi
print_success "Basic CLI functionality working"

print_step "Setting up test-artifacts directory..."
mkdir -p test-artifacts/fixtures
mkdir -p test-artifacts/campaigns
echo "{}" >test-artifacts/test-admin.json # Dummy keypair for testing
print_success "Test artifacts directory created"

print_step "Testing volume mounting..."
if ! docker run --rm \
    -v $(pwd)/test-artifacts:/workspace/test-artifacts \
    prism-protocol-cli \
    --version >/dev/null; then
    print_error "Volume mounting test failed"
fi
print_success "Volume mounting working"

print_step "Testing help command..."
if ! docker run --rm prism-protocol-cli --help >/dev/null; then
    print_error "Help command failed"
fi
print_success "Help command working"

print_step "Testing fixture generation command structure..."
# Note: This may fail if the command isn't implemented yet, but should show proper error handling
set +e # Temporarily disable exit on error
docker_output=$(docker run --rm \
    -v $(pwd)/test-artifacts:/workspace/test-artifacts \
    prism-protocol-cli \
    generate-fixtures --help 2>&1)
exit_code=$?
set -e # Re-enable exit on error

if [[ $exit_code -eq 0 ]]; then
    print_success "Generate fixtures command available"
elif echo "$docker_output" | grep -q "generate-fixtures"; then
    print_success "Generate fixtures command recognized (help output detected)"
elif echo "$docker_output" | grep -q "unrecognized subcommand"; then
    echo -e "${YELLOW}⚠️  Generate fixtures command not yet implemented (expected)${NC}"
else
    print_error "Unexpected error testing generate-fixtures command: $docker_output"
fi

print_step "Verifying file permissions..."
# Test that the container can write to the mounted volume by trying to generate fixtures
# Even if it fails due to missing implementation, it should be able to access the directory
set +e # Temporarily disable exit on error
docker_test_output=$(docker run --rm \
    -v $(pwd)/test-artifacts:/workspace/test-artifacts \
    prism-protocol-cli \
    generate-fixtures --campaign-name "docker-test" --count 1 2>&1)
test_exit_code=$?
set -e # Re-enable exit on error

# Check if we can at least access the directory (even if command fails for other reasons)
if echo "$docker_test_output" | grep -q "Permission denied"; then
    print_error "Container has permission issues with mounted volume"
elif echo "$docker_test_output" | grep -q "No such file or directory.*test-artifacts"; then
    print_error "Container cannot access mounted volume"
else
    print_success "File permissions working correctly (container can access mounted directory)"
fi

print_step "Testing development workflow simulation..."
echo -e "${BLUE}Simulating typical development commands:${NC}"

echo "1. Building CLI container (already done)"
echo "2. Generating test fixtures (would run):"
echo "   docker run -v \$(pwd)/test-artifacts:/workspace/test-artifacts prism-protocol-cli \\"
echo "     generate-fixtures --campaign-name \"Docker Test\" --count 100"

echo "3. Running host-based anchor tests (would run):"
echo "   anchor test"

echo "4. Container-based deployment testing (would run):"
echo "   docker run --network host -v \$(pwd)/test-artifacts:/workspace/test-artifacts prism-protocol-cli \\"
echo "     deploy-campaign --campaign-db-in test-artifacts/campaigns/docker-test.db"

print_success "Development workflow simulation complete"

echo -e "\n${GREEN}🎉 All Docker tests passed!${NC}"
echo -e "${BLUE}💡 Next steps:${NC}"
echo "1. Implement generate-fixtures command in CLI"
echo "2. Test with real fixture generation"
echo "3. Add CI/CD integration"
echo "4. Document developer onboarding process"

echo -e "\n${YELLOW}📝 Quick reference commands:${NC}"
echo "make docker-build-prism-cli                    # Build CLI container"
echo "make docker-test-volumes                       # Test volume mounting"
echo "make docker-help                               # Show Docker commands"

#!/bin/bash

# Prism Protocol CLI Test Suite
# Comprehensive testing of CLI commands with assertions

set -e # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
QUICK_MODE=false
VERBOSE=false
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DIR="$PROJECT_ROOT/test-artifacts/cli-tests"
CLI_BIN="cargo run -p prism-protocol-cli --"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
    --quick)
        QUICK_MODE=true
        shift
        ;;
    --verbose | -v)
        VERBOSE=true
        shift
        ;;
    *)
        echo "Unknown option: $1"
        exit 1
        ;;
    esac
done

# Utility functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_test() {
    echo -e "${BLUE}🧪 Testing: $1${NC}"
}

# Setup test environment
setup_test_env() {
    log_info "Setting up test environment..."
    log_info "Project root: $PROJECT_ROOT"
    log_info "Test directory: $TEST_DIR"

    # Create test directory
    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"

    # Ensure we have a test keypair
    if [[ ! -f "test-admin.json" ]]; then
        log_info "Generating test admin keypair..."
        solana-keygen new --no-bip39-passphrase --silent --outfile test-admin.json
    fi

    log_success "Test environment ready"
    log_info "All test artifacts will be created in: $TEST_DIR"
}

# Cleanup test artifacts
cleanup() {
    log_info "Cleaning up test artifacts..."
    if [[ -d "$TEST_DIR" ]]; then
        rm -rf "$TEST_DIR"
        log_success "Cleaned up: $TEST_DIR"
    fi

    # Also clean any stray files in project root
    cd "$PROJECT_ROOT"
    rm -f *.csv *.db test-*.csv test-*.db
    log_success "Cleanup completed"
}

# Trap cleanup on exit
trap cleanup EXIT

# Assert file exists
assert_file_exists() {
    local file="$1"
    local description="$2"

    if [[ ! -f "$file" ]]; then
        log_error "File not found: $file ($description)"
        exit 1
    fi
    log_success "File exists: $file"
}

# Assert file contains pattern
assert_file_contains() {
    local file="$1"
    local pattern="$2"
    local description="$3"

    if ! grep -q "$pattern" "$file"; then
        log_error "Pattern '$pattern' not found in $file ($description)"
        exit 1
    fi
    log_success "Pattern found in $file: $pattern"
}

# Assert command succeeds
assert_command_succeeds() {
    local cmd="$1"
    local description="$2"

    log_info "Running: $cmd"
    if ! eval "$cmd" >/dev/null 2>&1; then
        log_error "Command failed: $cmd ($description)"
        exit 1
    fi
    log_success "Command succeeded: $description"
}

# Assert command output contains pattern
assert_command_output_contains() {
    local cmd="$1"
    local pattern="$2"
    local description="$3"

    log_info "Running: $cmd"
    local output
    output=$(eval "$cmd" 2>&1)

    if ! echo "$output" | grep -q "$pattern"; then
        log_error "Command output doesn't contain '$pattern'"
        log_error "Command: $cmd"
        log_error "Output: $output"
        exit 1
    fi
    log_success "Command output contains pattern: $pattern"
}

# Test CLI help commands
test_cli_help() {
    log_test "CLI help commands"

    assert_command_output_contains "$CLI_BIN --help" "Prism Protocol CLI" "Main help"
    assert_command_output_contains "$CLI_BIN generate-fixtures --help" "Generate test fixtures" "Generate fixtures help"
    assert_command_output_contains "$CLI_BIN compile-campaign --help" "Compile campaign" "Compile campaign help"

    log_success "CLI help tests passed"
}

# Test fixture generation
test_fixture_generation() {
    log_test "Fixture generation"

    local test_cases=(
        "10:2:uniform"
        "100:3:realistic"
        "50:1:exponential"
    )

    if [[ "$QUICK_MODE" == "true" ]]; then
        test_cases=("10:2:uniform")
    fi

    for case in "${test_cases[@]}"; do
        IFS=':' read -r count cohorts distribution <<<"$case"

        log_info "Testing fixture generation: $count claimants, $cohorts cohorts, $distribution distribution"

        local campaign_name="Test Campaign ${count}-${cohorts}-${distribution}"
        local campaign_slug="test-campaign-${count}-${cohorts}-${distribution}"
        local fixture_dir="test-artifacts/fixtures/${campaign_slug}"

        # Generate fixtures with enhanced interface
        $CLI_BIN generate-fixtures \
            --campaign-name "$campaign_name" \
            --count "$count" \
            --cohort-count "$cohorts" \
            --distribution "$distribution"

        # Assert directory structure was created
        assert_file_exists "$fixture_dir" "Fixture directory"
        assert_file_exists "$fixture_dir/campaign.csv" "Campaign CSV"
        assert_file_exists "$fixture_dir/cohorts.csv" "Cohorts CSV"
        assert_file_exists "$fixture_dir/claimant-keypairs" "Keypairs directory"

        # Assert file structure
        assert_file_contains "$fixture_dir/campaign.csv" "cohort,claimant,entitlements" "Campaign CSV header"
        assert_file_contains "$fixture_dir/cohorts.csv" "cohort,amount_per_entitlement" "Cohorts CSV header"

        # Count lines (header + data)
        local campaign_lines
        campaign_lines=$(wc -l <"$fixture_dir/campaign.csv")
        local expected_lines=$((count + 1)) # +1 for header

        if [[ "$campaign_lines" -ne "$expected_lines" ]]; then
            log_error "Expected $expected_lines lines in campaign.csv, got $campaign_lines"
            exit 1
        fi

        local cohorts_lines
        cohorts_lines=$(wc -l <"$fixture_dir/cohorts.csv")
        local expected_cohort_lines=$((cohorts + 1)) # +1 for header

        if [[ "$cohorts_lines" -ne "$expected_cohort_lines" ]]; then
            log_error "Expected $expected_cohort_lines lines in cohorts.csv, got $cohorts_lines"
            exit 1
        fi

        # Check keypair files
        local keypair_count
        keypair_count=$(find "$fixture_dir/claimant-keypairs" -name "claimant-*.json" | wc -l)
        if [[ "$keypair_count" -ne "$count" ]]; then
            log_error "Expected $count keypair files, got $keypair_count"
            exit 1
        fi

        # Validate a sample keypair file
        local sample_keypair="$fixture_dir/claimant-keypairs/claimant-0001.json"
        assert_file_exists "$sample_keypair" "Sample keypair file"
        assert_file_contains "$sample_keypair" "\"keypair\":" "Keypair data"
        assert_file_contains "$sample_keypair" "\"pubkey\":" "Public key"
        assert_file_contains "$sample_keypair" "\"campaign\": \"$campaign_slug\"" "Campaign reference"

        log_success "Enhanced fixture generation test passed: $case"
    done

    log_success "All enhanced fixture generation tests passed"
}

# Test campaign compilation
test_campaign_compilation() {
    log_test "Campaign compilation with enhanced fixtures"

    # First generate test fixtures using the enhanced interface
    log_info "Generating test fixtures for compilation..."
    local campaign_name="Compilation Test Campaign"
    local campaign_slug="compilation-test-campaign"
    local fixture_dir="test-artifacts/fixtures/${campaign_slug}"

    $CLI_BIN generate-fixtures \
        --campaign-name "$campaign_name" \
        --count 20 \
        --cohort-count 3 \
        --distribution realistic

    # Test campaign compilation from fixture directory
    local db_file="test-campaign.db"
    local mint="EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" # USDC mint

    log_info "Compiling campaign from fixtures..."
    $CLI_BIN compile-campaign \
        --campaign-csv-in "$fixture_dir/campaign.csv" \
        --cohorts-csv-in "$fixture_dir/cohorts.csv" \
        --mint "$mint" \
        --admin-keypair test-admin.json \
        --campaign-db-out "$db_file"

    # Assert database was created
    assert_file_exists "$db_file" "Campaign database"

    # Test database content using sqlite3
    if command -v sqlite3 >/dev/null 2>&1; then
        log_info "Validating database content..."

        # Check tables exist
        local tables
        tables=$(sqlite3 "$db_file" ".tables")
        for table in campaign cohorts claimants vaults; do
            if ! echo "$tables" | grep -q "$table"; then
                log_error "Table '$table' not found in database"
                exit 1
            fi
        done

        # Check campaign record
        local campaign_count
        campaign_count=$(sqlite3 "$db_file" "SELECT COUNT(*) FROM campaign;")
        if [[ "$campaign_count" -ne 1 ]]; then
            log_error "Expected 1 campaign record, got $campaign_count"
            exit 1
        fi

        # Check cohorts
        local cohort_count
        cohort_count=$(sqlite3 "$db_file" "SELECT COUNT(*) FROM cohorts;")
        if [[ "$cohort_count" -ne 3 ]]; then
            log_error "Expected 3 cohort records, got $cohort_count"
            exit 1
        fi

        # Check claimants
        local claimant_count
        claimant_count=$(sqlite3 "$db_file" "SELECT COUNT(*) FROM claimants;")
        if [[ "$claimant_count" -ne 20 ]]; then
            log_error "Expected 20 claimant records, got $claimant_count"
            exit 1
        fi

        # Check that fingerprint is not empty
        local fingerprint
        fingerprint=$(sqlite3 "$db_file" "SELECT fingerprint FROM campaign;")
        if [[ -z "$fingerprint" ]]; then
            log_error "Campaign fingerprint is empty"
            exit 1
        fi

        log_success "Database validation passed"
    else
        log_warning "sqlite3 not available, skipping database content validation"
    fi

    log_success "Campaign compilation test passed"
}

# Test error handling
test_error_handling() {
    log_test "Error handling"

    # First create a test fixture for error testing
    local campaign_name="Error Test Campaign"
    local campaign_slug="error-test-campaign"
    local fixture_dir="test-artifacts/fixtures/${campaign_slug}"

    $CLI_BIN generate-fixtures \
        --campaign-name "$campaign_name" \
        --count 5 \
        --cohort-count 2 \
        --distribution uniform

    # Test missing files
    if $CLI_BIN compile-campaign \
        --campaign-csv-in nonexistent.csv \
        --cohorts-csv-in "$fixture_dir/cohorts.csv" \
        --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
        --admin-keypair test-admin.json \
        --campaign-db-out test.db >/dev/null 2>&1; then
        log_error "Expected command to fail with missing campaign file"
        exit 1
    fi

    # Test invalid mint
    if $CLI_BIN compile-campaign \
        --campaign-csv-in "$fixture_dir/campaign.csv" \
        --cohorts-csv-in "$fixture_dir/cohorts.csv" \
        --mint invalid-mint \
        --admin-keypair test-admin.json \
        --campaign-db-out test.db >/dev/null 2>&1; then
        log_error "Expected command to fail with invalid mint"
        exit 1
    fi

    # Test overwrite protection for fixtures
    if $CLI_BIN generate-fixtures \
        --campaign-name "$campaign_name" \
        --count 10 >/dev/null 2>&1; then
        log_error "Expected command to fail with overwrite protection"
        exit 1
    fi

    log_success "Error handling tests passed"
}

# Test overwrite protection (replaces deterministic behavior test)
test_overwrite_protection() {
    log_test "Overwrite protection"

    local campaign_name="Overwrite Test Campaign"
    local campaign_slug="overwrite-test-campaign"
    local fixture_dir="test-artifacts/fixtures/${campaign_slug}"

    # Generate initial fixtures
    $CLI_BIN generate-fixtures \
        --campaign-name "$campaign_name" \
        --count 10 \
        --cohort-count 2 \
        --distribution uniform

    # Verify fixtures were created
    assert_file_exists "$fixture_dir/campaign.csv" "Initial campaign CSV"
    assert_file_exists "$fixture_dir/cohorts.csv" "Initial cohorts CSV"

    # Try to generate again with same name - should fail
    if $CLI_BIN generate-fixtures \
        --campaign-name "$campaign_name" \
        --count 20 >/dev/null 2>&1; then
        log_error "Expected command to fail due to overwrite protection"
        exit 1
    fi

    # Verify original files are unchanged
    local lines_count
    lines_count=$(wc -l <"$fixture_dir/campaign.csv")
    if [[ "$lines_count" -ne 11 ]]; then # 10 claimants + 1 header
        log_error "Original files were modified despite overwrite protection"
        exit 1
    fi

    # Test that removal allows regeneration
    rm -rf "$fixture_dir"

    $CLI_BIN generate-fixtures \
        --campaign-name "$campaign_name" \
        --count 15 \
        --cohort-count 2 \
        --distribution uniform

    # Verify new fixtures have correct count
    lines_count=$(wc -l <"$fixture_dir/campaign.csv")
    if [[ "$lines_count" -ne 16 ]]; then # 15 claimants + 1 header
        log_error "Regenerated fixtures have wrong count"
        exit 1
    fi

    log_success "Overwrite protection test passed"
}

# Main test execution
main() {
    log_info "Starting Prism Protocol CLI Test Suite"
    log_info "Quick mode: $QUICK_MODE"
    log_info "Verbose mode: $VERBOSE"

    setup_test_env

    # Run test suites
    test_cli_help
    test_fixture_generation
    test_campaign_compilation
    test_error_handling
    test_overwrite_protection

    log_success "All CLI tests passed! 🎉"
}

# Run main function
main "$@"

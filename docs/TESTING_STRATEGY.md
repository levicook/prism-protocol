# Prism Protocol Testing Strategy

## Overview

Our testing approach is built around two main categories:
1. **Happy Path Testing** - Normal expected workflows
2. **Deviant Path Testing** - Error conditions and edge cases to exercise every error code

## Testing Framework

We use our custom state machine testing framework built on Mollusk that allows:
- Starting tests at any campaign lifecycle stage
- Comprehensive real SPL token operations with Merkle trees
- Fast execution (~1 second for full test suite)
- Deterministic test data and addresses

## Test Organization

**Note**: `cargo test-sbf` requires integration test files to be directly in the `tests/` directory, not in subdirectories. Each `.rs` file becomes a separate integration test binary. We use descriptive file names to organize by category.

### 1. Happy Path Tests
Complete end-to-end journeys following normal user workflows:

- **`test_full_campaign_flow.rs`** ✅ 
  - Complete campaign creation to token claiming
  - Multiple cohorts with different configurations
  - Real vault funding and token operations

### 2. Error Conditions Tests
Systematic testing of all error scenarios:

- **`test_premature_actions.rs`** ✅
  - Actions attempted before proper setup
  - Out-of-bounds operations
  
- **`test_authority_errors.rs`** ✅
  - Wrong admin attempting actions
  - Non-admin trying admin operations
  
- **`test_validation_errors.rs`** ✅
  - Invalid merkle roots
  - Mismatched fingerprints
  - Invalid amounts

- **`test_constraint_errors.rs`** 🚧 TODO
  - Address derivation mismatches
  - Account ownership violations

### 3. Campaign Lifecycle Tests
Campaign-specific state transitions and operations:

- **`test_campaign_activation.rs`** 🚧 TODO
  - Campaign status transitions
  - Unstoppable vs stoppable campaigns
  - Pause/resume functionality

- **`test_campaign_finalization.rs`** 🚧 TODO
  - Token reclamation flows
  - Campaign closure scenarios

### 4. Cohort Lifecycle Tests
Cohort-specific operations and validations:

- **`test_cohort_creation.rs`** 🚧 TODO
  - Various cohort configurations
  - Merkle tree validation
  - Expected vault count enforcement

- **`test_cohort_state_changes.rs`** 🚧 TODO
  - Cohort activation flows
  - Cohort-specific error conditions

### 5. Vault Operations Tests
Vault creation, funding, and management:

- **`test_vault_creation.rs`** 🚧 TODO
  - Vault initialization with different indexes
  - SPL token account setup
  - PDA derivation validation

- **`test_vault_funding.rs`** 🚧 TODO
  - Token minting to vaults
  - Funding validation
  - Insufficient funding scenarios

### 6. Claim Flow Tests
Token claiming with Merkle proof validation:

- **`test_claim_valid.rs`** 🚧 TODO
  - Single and multiple claims
  - Different entitlement amounts
  - Cross-vault claiming

- **`test_claim_invalid.rs`** 🚧 TODO
  - Invalid Merkle proofs
  - Double claiming attempts
  - Claims with wrong amounts

## Error Code Coverage Goals

We aim to test every error code in our `PrismProtocolError` enum (32 total):

### Basic Validation Errors
- [ ] `InvalidMerkleProof`
- [ ] `MerkleRootMismatch`
- [ ] `NumericOverflow`
- [ ] `InvalidEntitlements`

### Authorization and Access Errors
- [ ] `TokenAccountOwnerMismatch`
- [ ] `CampaignAdminMismatch`

### PDA/Constraint Validation Errors
- [ ] `CampaignFingerprintMismatch`
- [ ] `CohortCampaignMismatch`
- [ ] `MintMismatch`

### Campaign Lifecycle Errors
- [ ] `CampaignNotActive`
- [ ] `CampaignIsActive`
- [ ] `CampaignAlreadyActivated`
- [ ] `CampaignIsUnstoppable`
- [ ] `CampaignNotPaused`
- [ ] `CampaignNotPermanentlyHalted`
- [ ] `InvalidStatusTransition`
- [ ] `GoLiveDateNotReached`

### Campaign Setup/Activation Errors
- [ ] `InvalidIpfsHash`
- [ ] `GoLiveSlotInPast`
- [ ] `NoCohortsExpected`
- [ ] `NotAllCohortsActivated`

### Cohort and Vault Setup Errors
- [ ] `NoVaultsExpected`
- [ ] `VaultIndexOutOfBounds`
- [ ] `TooManyVaults`
- [ ] `VaultNotInitialized`
- [ ] `IncorrectVaultFunding`
- [ ] `NotAllVaultsActivated`

### Claiming Errors
- [ ] `AssignedVaultIndexOutOfBounds`

## Testing Utilities

### TestFixture
Our main testing harness providing:
- Campaign setup to any lifecycle stage
- Real SPL token operations
- Deterministic test data generation
- Expectation helpers (`expect_success`, `expect_failure`)

### TestClaimants
Well-known test identities:
- Alice, Bob, Charlie, Diana, Eve
- Consistent across all tests
- Various group sizes (small, medium, all)

### State Machine Approach
- `CampaignLifecycleStage` enum for precise state control
- `CampaignAction` enum for action testing
- `setup_to_stage()` for test preparation
- `advance_to_stage()` for progression testing

## Implementation Priority

1. **Complete Error Coverage** - Implement all error condition tests
2. **Campaign Lifecycle** - State transition testing
3. **Claim Flows** - Merkle proof and claiming scenarios
4. **Vault Operations** - Comprehensive vault testing  
5. **Cohort Lifecycle** - Cohort-specific functionality

## Success Metrics

- [ ] All 32 error codes tested
- [ ] All campaign lifecycle stages covered
- [ ] All instructions have happy path + error tests
- [ ] Test suite runs in under 2 seconds
- [ ] 100% instruction coverage
- [ ] Clear test failure messages

## Running Tests

```bash
# Run all tests
cargo test-sbf

# Run specific category
cargo test-sbf happy_path
cargo test-sbf error_conditions

# Run with output
cargo test-sbf -- --nocapture
```

This strategy ensures comprehensive coverage while maintaining fast, reliable tests that give us confidence in the protocol's robustness. 
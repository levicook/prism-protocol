# Transaction Architecture Specification

## Overview

This specification outlines the redesign of Prism Protocol's transaction building, packing, and execution system to move from ad-hoc "build and send" patterns to a structured "collect instructions → pack transactions → transmit" architecture.

## Current Problems

### Scattered Transaction Logic

- **deploy_campaign.rs**: 50+ individual transactions with duplicate transaction building code
- **claim_tokens.rs**: Separate transactions per cohort for multi-cohort claimants
- **No batching optimization**: Each instruction becomes its own transaction
- **Inconsistent retry logic**: Different error handling patterns across commands

### Raw RPC Client Usage

Commands frequently bypass PrismProtocolClient for basic operations:

- `rpc_client.get_latest_blockhash()` - 8+ occurrences in deploy_campaign.rs
- `rpc_client.send_and_confirm_transaction_with_spinner_and_config()` - 6+ occurrences
- Manual account existence checking via `rpc_client.get_account()`
- Direct SOL balance queries and connection testing

### Deployment State Management

- **No coordination**: Commands don't know what's already deployed
- **No recovery**: Failed deployments require manual cleanup
- **No auditability**: Deployment signatures scattered in logs

## Proposed Architecture

### 1. Deploy Planner Abstraction

**Purpose**: Determine what instructions need to be built based on current deployment state.

```rust
pub struct DeployPlanner {
    db: CampaignDatabase,
    client: PrismProtocolClient,
}

pub struct DeploymentPlan {
    pub steps: Vec<DeploymentStep>,
    pub estimated_transactions: usize,
    pub estimated_cost: u64, // in lamports
}

pub enum DeploymentStep {
    InitializeCampaign { fingerprint: [u8; 32] },
    InitializeCohort { cohort_name: String, merkle_root: [u8; 32] },
    CreateVault { cohort_name: String, vault_index: u8 },
    FundVault { cohort_name: String, vault_index: u8, amount: u64 },
    ActivateCampaign { fingerprint: [u8; 32] },
}

impl DeployPlanner {
    pub fn create_deployment_plan(&self, fingerprint: &[u8; 32]) -> Result<DeploymentPlan, PlannerError> {
        // 1. Check database for existing deployment signatures
        let db_status = self.db.get_deployment_status(fingerprint)?;

        // 2. Cross-reference with on-chain state (surgical account queries)
        let onchain_status = self.client.get_campaign_deployment_status(fingerprint)?;

        // 3. Generate instruction plan for missing/incomplete deployments
        let mut steps = Vec::new();

        if !db_status.campaign_deployed && !onchain_status.campaign_exists {
            steps.push(DeploymentStep::InitializeCampaign { fingerprint: *fingerprint });
        }

        // ... similar logic for cohorts, vaults, funding, activation

        Ok(DeploymentPlan { steps, estimated_transactions: 0, estimated_cost: 0 })
    }
}
```

**Benefits**:

- **Idempotent deployments**: Can be run multiple times safely
- **Partial recovery**: Resume failed deployments from any point
- **Cost estimation**: Know transaction costs before execution
- **Clear state visibility**: Understand what's deployed vs. what's needed

### 2. Generic Instruction/Transaction Packer

**Purpose**: Pack instructions into optimally-sized transactions while respecting dependencies.

```rust
pub struct TransactionPacker {
    max_transaction_size: usize, // ~1232 bytes typical limit
    max_instructions_per_tx: usize, // Conservative limit
}

pub struct PackedTransaction {
    pub transaction: Transaction,
    pub instructions: Vec<Instruction>,
    pub signers: Vec<Keypair>, // Owned signers for this transaction
}

impl TransactionPacker {
    pub fn pack_instructions(
        &self,
        instructions: Vec<Instruction>,
        payer: &Keypair,
        additional_signers: &[&Keypair],
        recent_blockhash: Hash,
    ) -> Result<Vec<PackedTransaction>, PackerError> {
        let mut packed_txs = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_size = 0;

        for instruction in instructions {
            let estimated_size = self.estimate_instruction_size(&instruction);

            // Check if adding this instruction would exceed limits
            if current_size + estimated_size > self.max_transaction_size
                || current_batch.len() >= self.max_instructions_per_tx {

                // Pack current batch into transaction
                if !current_batch.is_empty() {
                    packed_txs.push(self.build_transaction(
                        current_batch.drain(..).collect(),
                        payer,
                        additional_signers,
                        recent_blockhash,
                    )?);
                    current_size = 0;
                }
            }

            current_batch.push(instruction);
            current_size += estimated_size;
        }

        // Pack remaining instructions
        if !current_batch.is_empty() {
            packed_txs.push(self.build_transaction(
                current_batch,
                payer,
                additional_signers,
                recent_blockhash,
            )?);
        }

        Ok(packed_txs)
    }

    fn estimate_instruction_size(&self, instruction: &Instruction) -> usize {
        // Conservative estimation based on accounts, data size, etc.
        32 * instruction.accounts.len() + instruction.data.len() + 64 // overhead
    }
}
```

**Usage Examples**:

```rust
// Deployment packing
let instructions = deployment_plan.build_instructions()?;
let transactions = packer.pack_instructions(instructions, &admin_keypair, &[], blockhash)?;

// Claim packing (multi-cohort)
let claim_instructions = build_claim_instructions_for_claimant(&db, &claimant)?;
let claim_transactions = packer.pack_instructions(claim_instructions, &claimant, &[], blockhash)?;
```

### 3. Unified Transaction Retry Utility

**Purpose**: Single retry mechanism with proper re-signing and error handling.

```rust
pub struct TransactionRetryConfig {
    pub max_retries: u8,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub confirmation_timeout_seconds: u64,
}

pub struct TransactionTransmitter {
    rpc_client: Arc<RpcClient>,
    config: TransactionRetryConfig,
}

impl TransactionTransmitter {
    pub async fn transmit_transactions(
        &self,
        transactions: Vec<PackedTransaction>,
        admin_keypair: &Keypair,
    ) -> Result<Vec<Signature>, TransmissionError> {
        let mut signatures = Vec::new();

        for (i, mut packed_tx) in transactions.into_iter().enumerate() {
            println!("📦 Transmitting transaction {} of {}", i + 1, signatures.len() + 1);

            let signature = self.transmit_with_retry(&mut packed_tx, admin_keypair).await?;
            signatures.push(signature);

            println!("✅ Transaction confirmed: https://explorer.solana.com/tx/{}", signature);
        }

        Ok(signatures)
    }

    async fn transmit_with_retry(
        &self,
        packed_tx: &mut PackedTransaction,
        admin_keypair: &Keypair,
    ) -> Result<Signature, TransmissionError> {
        for attempt in 0..self.config.max_retries {
            // Fresh blockhash for each retry (critical!)
            let recent_blockhash = self.rpc_client.get_latest_blockhash()
                .map_err(TransmissionError::BlockhashFetch)?;

            // Re-sign transaction with fresh blockhash
            packed_tx.transaction.message.recent_blockhash = recent_blockhash;
            packed_tx.transaction.sign(&[admin_keypair], recent_blockhash);

            // Attempt transmission
            match self.rpc_client.send_transaction(&packed_tx.transaction) {
                Ok(signature) => {
                    // Wait for confirmation
                    match self.wait_for_confirmation(&signature).await {
                        Ok(()) => return Ok(signature),
                        Err(e) => {
                            println!("⚠️ Attempt {} failed during confirmation: {}", attempt + 1, e);
                            if attempt == self.config.max_retries - 1 {
                                return Err(TransmissionError::ConfirmationTimeout(signature));
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️ Attempt {} failed during transmission: {}", attempt + 1, e);
                    if attempt == self.config.max_retries - 1 {
                        return Err(TransmissionError::TransmissionFailed(e));
                    }
                }
            }

            // Exponential backoff with jitter
            let delay = std::cmp::min(
                self.config.base_delay_ms * 2_u64.pow(attempt as u32),
                self.config.max_delay_ms,
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        }

        unreachable!("Should have returned or errored within retry loop")
    }
}
```

### 4. Database Deployment Coordinator

**Purpose**: Track deployment signatures and provide state queries for recovery.

```rust
impl CampaignDatabase {
    // Deployment tracking
    pub fn mark_campaign_deployed(&mut self, fingerprint: &[u8; 32], signature: &str) -> DbResult<()>
    pub fn mark_cohort_deployed(&mut self, cohort_name: &str, signature: &str) -> DbResult<()>
    pub fn mark_vault_created(&mut self, cohort_name: &str, vault_index: u8, signature: &str) -> DbResult<()>
    pub fn mark_vault_funded(&mut self, cohort_name: &str, vault_index: u8, signature: &str, amount: u64) -> DbResult<()>
    pub fn mark_campaign_activated(&mut self, fingerprint: &[u8; 32], signature: &str) -> DbResult<()>

    // State queries
    pub fn get_deployment_status(&self, fingerprint: &[u8; 32]) -> DbResult<DeploymentStatus>
    pub fn get_missing_deployments(&self, fingerprint: &[u8; 32]) -> DbResult<Vec<DeploymentStep>>

    // Recovery operations
    pub fn reset_deployment_state(&mut self, fingerprint: &[u8; 32]) -> DbResult<()> // For testing
    pub fn get_deployment_audit_log(&self, fingerprint: &[u8; 32]) -> DbResult<Vec<DeploymentLogEntry>>
}

pub struct DeploymentStatus {
    pub campaign_deployed: bool,
    pub cohorts_deployed: Vec<String>, // Names of deployed cohorts
    pub vaults_created: HashMap<String, Vec<u8>>, // cohort_name -> vault_indices
    pub vaults_funded: HashMap<String, Vec<u8>>, // cohort_name -> funded_vault_indices
    pub campaign_activated: bool,
}
```

## Integration Example

**Enhanced deploy_campaign.rs**:

```rust
pub fn execute(campaign_db_in: PathBuf, admin_keypair: PathBuf, rpc_url: String) -> CliResult<()> {
    // 1. Setup
    let admin_keypair = read_keypair_file(&admin_keypair)?;
    let rpc_client = Arc::new(RpcClient::new_with_commitment(&rpc_url, CommitmentConfig::confirmed()));
    let client = PrismProtocolClient::new(rpc_client.clone());
    let mut db = CampaignDatabase::open(&campaign_db_in)?;

    // 2. Planning phase
    let planner = DeployPlanner::new(db, client);
    let deployment_plan = planner.create_deployment_plan(&campaign_fingerprint)?;

    println!("📋 Deployment plan: {} steps, ~{} transactions, ~${:.2} cost",
        deployment_plan.steps.len(),
        deployment_plan.estimated_transactions,
        deployment_plan.estimated_cost as f64 / 1e9
    );

    // 3. Instruction building phase
    let instructions = deployment_plan.build_instructions()?;

    // 4. Transaction packing phase
    let packer = TransactionPacker::new();
    let recent_blockhash = client.get_latest_blockhash()?;
    let packed_transactions = packer.pack_instructions(
        instructions,
        &admin_keypair,
        &[], // No additional signers for deployment
        recent_blockhash,
    )?;

    println!("📦 Packed {} instructions into {} transactions",
        instructions.len(), packed_transactions.len());

    // 5. Transmission phase
    let transmitter = TransactionTransmitter::new(rpc_client, TransactionRetryConfig::default());
    let signatures = transmitter.transmit_transactions(packed_transactions, &admin_keypair).await?;

    // 6. Database coordination phase
    for (step, signature) in deployment_plan.steps.iter().zip(signatures.iter()) {
        match step {
            DeploymentStep::InitializeCampaign { fingerprint } => {
                db.mark_campaign_deployed(fingerprint, &signature.to_string())?;
            }
            DeploymentStep::InitializeCohort { cohort_name, .. } => {
                db.mark_cohort_deployed(cohort_name, &signature.to_string())?;
            }
            // ... handle other steps
        }
    }

    println!("🎉 Deployment completed successfully!");
    println!("📊 Summary: {} transactions, {:.2}s total time", signatures.len(), total_time);

    Ok(())
}
```

## Expected Impact

### Performance Improvements

- **50+ transactions → 3-5 transactions** (deployment)
- **~60 seconds → ~10 seconds** deployment time
- **~$1.25 → ~$0.15** in transaction fees
- **5+ transactions → 1-2 transactions** (multi-cohort claims)

### Reliability Improvements

- **Idempotent deployments**: Can be run multiple times safely
- **Automatic recovery**: Resume from any failure point
- **Consistent retry logic**: Proper re-signing and error handling
- **Complete auditability**: All signatures tracked in database

### Developer Experience

- **Clean separation of concerns**: Planning → Packing → Transmission
- **Reusable components**: Generic packer works for deploy, claims, any multi-instruction operation
- **Better error messages**: Clear indication of what failed and how to recover
- **Simplified CLI commands**: Business logic separated from transaction mechanics

## Implementation Plan

### Phase 1: Core Infrastructure (3-4 days)

1. **Deploy Planner**: State analysis and instruction planning
2. **Transaction Packer**: Generic instruction batching utility
3. **Retry Transmitter**: Unified retry mechanism with re-signing
4. **Database Coordinator**: Deployment state tracking

### Phase 2: Integration (2-3 days)

1. **Enhanced PrismProtocolClient**: Add missing abstractions to eliminate raw RPC usage
2. **Update deploy_campaign.rs**: Use new architecture
3. **Update claim_tokens.rs**: Use new packing for multi-cohort claims

### Phase 3: Validation (1-2 days)

1. **End-to-end testing**: Verify improved performance and reliability
2. **Failure recovery testing**: Ensure proper recovery from various failure scenarios
3. **Documentation**: Update CLI documentation and examples

**Total Estimated Effort**: 6-9 days

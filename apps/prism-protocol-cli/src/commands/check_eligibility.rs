use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_client::PrismProtocolClient;
use prism_protocol_db::CampaignDatabase;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
};
use std::str::FromStr;
use std::{path::PathBuf, sync::Arc};

// Extended eligibility info with on-chain state
#[derive(Debug)]
#[allow(dead_code)]
struct EligibilityInfo {
    // From database
    cohort_name: String,
    cohort_merkle_root: [u8; 32],
    entitlements: u64,
    amount_per_entitlement: u64,
    total_tokens: u64,
    db_claimed: bool,
    db_claimed_at: Option<i64>,
    db_claimed_signature: Option<String>,
    // On-chain verification
    onchain_claimed: Option<bool>, // None = not checked, Some(true/false) = checked
    claim_receipt_address: Pubkey,
}

pub fn execute(campaign_db_in: PathBuf, claimant: String, rpc_url: String) -> CliResult<()> {
    // Auto-detect whether claimant is a pubkey or keypair file
    let claimant_pubkey = if let Ok(pubkey) = Pubkey::from_str(&claimant) {
        // It's a valid pubkey string
        println!("🔍 Using provided pubkey: {}", pubkey);
        pubkey
    } else {
        // Assume it's a file path to a keypair
        let keypair_path = PathBuf::from(&claimant);
        println!("🔍 Reading keypair from: {}", keypair_path.display());
        let keypair = read_keypair_file(&keypair_path).map_err(|e| {
            CliError::InvalidConfig(format!("Failed to read keypair from '{}': {}", claimant, e))
        })?;
        let pubkey = keypair.pubkey();
        println!("🔑 Derived pubkey: {}", pubkey);
        pubkey
    };

    println!("🔍 Checking eligibility for claimant: {}", claimant_pubkey);

    // Open database and read campaign info using our new interface
    println!("📊 Reading campaign information...");
    let db = CampaignDatabase::open(&campaign_db_in)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to open database: {}", e)))?;

    let campaign_info = db
        .read_campaign_info()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read campaign info: {}", e)))?;

    println!("✅ Campaign: {}", hex::encode(campaign_info.fingerprint));
    println!("   Mint: {}", campaign_info.mint);
    println!("   Admin: {}", campaign_info.admin);

    // Query eligibility from database using our new interface
    println!("\n🔍 Querying database eligibility...");
    let db_eligibility = db
        .read_claimant_eligibility(&claimant_pubkey)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to query eligibility: {}", e)))?;

    if db_eligibility.is_empty() {
        println!("❌ No eligibility found for claimant {}", claimant_pubkey);
        println!("   This claimant is not part of this campaign.");
        return Ok(());
    }

    // Create RPC client using our new interface
    println!("🌐 Verifying on-chain claim status...");

    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        &rpc_url,
        CommitmentConfig::confirmed(),
    ));

    let client = PrismProtocolClient::new(rpc_client.clone());

    // Convert database eligibility to extended format with on-chain verification
    let mut eligibility: Vec<EligibilityInfo> = Vec::new();
    for db_entry in db_eligibility {
        // Calculate claim receipt address using the client's address finder
        let (campaign_address, _) = client
            .address_finder()
            .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);

        let (cohort_address, _) = client
            .address_finder()
            .find_cohort_v0_address(&campaign_address, &db_entry.cohort_merkle_root);

        let (claim_receipt_address, _) = client
            .address_finder()
            .find_claim_receipt_v0_address(&cohort_address, &claimant_pubkey);

        // Check on-chain claim receipt
        let onchain_claimed = match client.get_claim_receipt_v0(&cohort_address, &claimant_pubkey) {
            Ok(Some(_)) => Some(true),
            Ok(None) => Some(false),
            Err(_) => Some(false), // Treat RPC errors as "not claimed"
        };

        eligibility.push(EligibilityInfo {
            cohort_name: db_entry.cohort_name,
            cohort_merkle_root: db_entry.cohort_merkle_root,
            entitlements: db_entry.entitlements,
            amount_per_entitlement: db_entry.amount_per_entitlement,
            total_tokens: db_entry.total_tokens,
            db_claimed: db_entry.db_claimed,
            db_claimed_at: db_entry.db_claimed_at,
            db_claimed_signature: db_entry.db_claimed_signature,
            onchain_claimed,
            claim_receipt_address,
        });
    }

    // Display results with on-chain verification (IDENTICAL output format)
    println!("✅ Found eligibility in {} cohort(s):\n", eligibility.len());

    let mut total_claimable = 0u64;
    let mut total_claimed = 0u64;
    let mut unclaimed_count = 0;
    let mut db_onchain_mismatches = 0;

    for (i, entry) in eligibility.iter().enumerate() {
        println!("{}. Cohort: {}", i + 1, entry.cohort_name);
        println!("   Entitlements: {}", entry.entitlements);
        println!(
            "   Amount per entitlement: {} tokens",
            entry.amount_per_entitlement
        );
        println!("   Total tokens: {} tokens", entry.total_tokens);

        let onchain_claimed = entry.onchain_claimed.unwrap_or(false);

        // Check for database vs on-chain mismatches
        if entry.db_claimed != onchain_claimed {
            db_onchain_mismatches += 1;
            println!("   ⚠️  STATUS MISMATCH:");
            println!(
                "      Database: {}",
                if entry.db_claimed {
                    "CLAIMED"
                } else {
                    "UNCLAIMED"
                }
            );
            println!(
                "      On-chain: {}",
                if onchain_claimed {
                    "CLAIMED"
                } else {
                    "UNCLAIMED"
                }
            );
            println!("      Claim receipt: {}", entry.claim_receipt_address);
        } else if onchain_claimed {
            println!("   Status: ✅ CLAIMED (verified on-chain)");
            if let Some(claimed_at) = entry.db_claimed_at {
                let datetime = chrono::DateTime::from_timestamp(claimed_at, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Invalid timestamp".to_string());
                println!("   Claimed at: {}", datetime);
            }
            if let Some(signature) = &entry.db_claimed_signature {
                println!("   Transaction: {}", signature);
            }
            total_claimed += entry.total_tokens;
        } else {
            println!("   Status: ⏳ CLAIMABLE (verified on-chain)");
            total_claimable += entry.total_tokens;
            unclaimed_count += 1;
        }
        println!();
    }

    // Summary with verification results (IDENTICAL output format)
    println!("📊 Summary:");
    println!("   Total cohorts: {}", eligibility.len());
    println!("   Unclaimed cohorts: {}", unclaimed_count);
    println!("   Claimable tokens: {} tokens", total_claimable);
    if total_claimed > 0 {
        println!("   Already claimed: {} tokens", total_claimed);
    }

    if db_onchain_mismatches > 0 {
        println!(
            "   ⚠️  Database/on-chain mismatches: {}",
            db_onchain_mismatches
        );
        println!("      (On-chain state is authoritative)");
    } else {
        println!("   ✅ Database and on-chain state are consistent");
    }

    if total_claimable > 0 {
        println!("\n💡 To claim your tokens, run:");
        println!("   prism-protocol claim-tokens \\");
        println!("     --campaign-db-in {} \\", campaign_db_in.display());
        println!("     --claimant-keypair <YOUR_KEYPAIR_FILE> \\");
        println!("     --rpc-url <RPC_URL>");
    }

    Ok(())
}

use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_sdk::address_finders::{
    find_campaign_address, find_claim_receipt_v0_address, find_cohort_v0_address,
};
use rusqlite::Connection;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
struct CampaignInfo {
    fingerprint: [u8; 32],
    mint: Pubkey,
    admin: Pubkey,
}

#[derive(Debug)]
#[allow(dead_code)]
struct EligibilityInfo {
    cohort_name: String,
    cohort_merkle_root: [u8; 32],
    entitlements: u64,
    amount_per_entitlement: u64,
    total_tokens: u64,
    // Database state
    db_claimed: bool,
    db_claimed_at: Option<i64>,
    db_claimed_signature: Option<String>,
    // On-chain state
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

    // Read campaign info
    println!("📊 Reading campaign information...");
    let campaign_info = read_campaign_info(&campaign_db_in)?;
    println!("✅ Campaign: {}", hex::encode(campaign_info.fingerprint));
    println!("   Mint: {}", campaign_info.mint);
    println!("   Admin: {}", campaign_info.admin);

    // Query eligibility from database
    println!("\n🔍 Querying database eligibility...");
    let mut eligibility =
        query_claimant_eligibility(&campaign_db_in, &claimant_pubkey, &campaign_info)?;

    if eligibility.is_empty() {
        println!("❌ No eligibility found for claimant {}", claimant_pubkey);
        println!("   This claimant is not part of this campaign.");
        return Ok(());
    }

    // Verify on-chain state
    println!("🌐 Verifying on-chain claim status...");
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    for entry in &mut eligibility {
        match rpc_client.get_account(&entry.claim_receipt_address) {
            Ok(_) => {
                entry.onchain_claimed = Some(true);
            }
            Err(_) => {
                entry.onchain_claimed = Some(false);
            }
        }
    }

    // Display results with on-chain verification
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

    // Summary with verification results
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

fn read_campaign_info(db_path: &PathBuf) -> CliResult<CampaignInfo> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare("SELECT fingerprint, mint, admin FROM campaign LIMIT 1")?;
    let mut rows = stmt.query_map([], |row| {
        let fingerprint_hex: String = row.get(0)?;
        let mint_str: String = row.get(1)?;
        let admin_str: String = row.get(2)?;
        Ok((fingerprint_hex, mint_str, admin_str))
    })?;

    if let Some(row) = rows.next() {
        let (fingerprint_hex, mint_str, admin_str) = row?;

        let fingerprint_bytes = hex::decode(fingerprint_hex)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid fingerprint hex: {}", e)))?;
        let fingerprint: [u8; 32] = fingerprint_bytes
            .try_into()
            .map_err(|_| CliError::InvalidConfig("Fingerprint must be 32 bytes".to_string()))?;

        let mint = Pubkey::from_str(&mint_str)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid mint pubkey: {}", e)))?;

        let admin = Pubkey::from_str(&admin_str)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid admin pubkey: {}", e)))?;

        Ok(CampaignInfo {
            fingerprint,
            mint,
            admin,
        })
    } else {
        Err(CliError::InvalidConfig(
            "No campaign data found in database".to_string(),
        ))
    }
}

fn query_claimant_eligibility(
    db_path: &PathBuf,
    claimant: &Pubkey,
    campaign_info: &CampaignInfo,
) -> CliResult<Vec<EligibilityInfo>> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT 
            c.cohort_name,
            c.entitlements,
            c.claimed_at,
            c.claimed_signature,
            h.amount_per_entitlement,
            h.merkle_root
         FROM claimants c
         JOIN cohorts h ON c.cohort_name = h.cohort_name
         WHERE c.claimant = ?
         ORDER BY c.cohort_name",
    )?;

    let rows = stmt.query_map([claimant.to_string()], |row| {
        let cohort_name: String = row.get(0)?;
        let entitlements: u64 = row.get(1)?;
        let claimed_at: Option<i64> = row.get(2)?;
        let claimed_signature: Option<String> = row.get(3)?;
        let amount_per_entitlement: u64 = row.get(4)?;
        let merkle_root_hex: String = row.get(5)?;

        Ok((
            cohort_name,
            entitlements,
            claimed_at,
            claimed_signature,
            amount_per_entitlement,
            merkle_root_hex,
        ))
    })?;

    let mut eligibility = Vec::new();
    for row in rows {
        let (
            cohort_name,
            entitlements,
            claimed_at,
            claimed_signature,
            amount_per_entitlement,
            merkle_root_hex,
        ) = row?;

        // Parse merkle root
        let merkle_root_bytes = hex::decode(merkle_root_hex)
            .map_err(|e| CliError::InvalidConfig(format!("Invalid merkle root hex: {}", e)))?;
        let cohort_merkle_root: [u8; 32] = merkle_root_bytes
            .try_into()
            .map_err(|_| CliError::InvalidConfig("Merkle root must be 32 bytes".to_string()))?;

        // Calculate claim receipt address
        let (campaign_address, _) =
            find_campaign_address(&campaign_info.admin, &campaign_info.fingerprint);
        let (cohort_address, _) = find_cohort_v0_address(&campaign_address, &cohort_merkle_root);
        let (claim_receipt_address, _) = find_claim_receipt_v0_address(&cohort_address, claimant);

        let total_tokens = entitlements * amount_per_entitlement;
        let already_claimed = claimed_at.is_some();

        eligibility.push(EligibilityInfo {
            cohort_name,
            cohort_merkle_root,
            entitlements,
            amount_per_entitlement,
            total_tokens,
            // Database state
            db_claimed: already_claimed,
            db_claimed_at: claimed_at,
            db_claimed_signature: claimed_signature,
            // On-chain state
            onchain_claimed: None,
            claim_receipt_address,
        });
    }

    Ok(eligibility)
}

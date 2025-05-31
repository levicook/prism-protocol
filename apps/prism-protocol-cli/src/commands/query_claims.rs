use crate::error::{CliError, CliResult};
use chrono;
use hex;
use prism_protocol_client::PrismProtocolClient;
use prism_protocol_db::CampaignDatabase;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
struct ClaimInfo {
    claim_receipt_address: Pubkey,
    cohort_address: Pubkey,
    cohort_name: String,
    claimant: Pubkey,
    assigned_vault: Pubkey,
    claimed_at_timestamp: i64,
}

pub fn execute(campaign_db_path: PathBuf, claimant: String, rpc_url: String) -> CliResult<()> {
    println!("⛓️  Querying blockchain for claims...");

    // Open database and create unified client
    let db = CampaignDatabase::open(&campaign_db_path)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to open database: {}", e)))?;
    let client = PrismProtocolClient::new(rpc_url)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to create RPC client: {}", e)))?;

    // Auto-detect whether claimant is a pubkey or keypair file
    let claimant_pubkey = parse_claimant_input(&claimant)?;
    println!("🔍 Querying claims for: {}", claimant_pubkey);

    // Get campaign info
    println!("📊 Reading campaign information...");
    let campaign_info = db
        .read_campaign_info()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read campaign: {}", e)))?;
    println!("✅ Campaign: {}", hex::encode(campaign_info.fingerprint));
    println!("   Mint: {}", campaign_info.mint);
    println!("   Admin: {}", campaign_info.admin);

    // Calculate campaign address
    let (campaign_address, _) = client
        .address_finder()
        .find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);
    println!("🏛️  Campaign address: {}", campaign_address);

    // Query blockchain for actual claim receipts
    println!("\n🌐 Querying blockchain for claim receipts...");
    let claims = query_claim_receipts(&db, &client, &campaign_address, &claimant_pubkey)?;

    // Display results
    display_claims_results(&claims, &campaign_info.fingerprint);

    Ok(())
}

fn parse_claimant_input(claimant: &str) -> CliResult<Pubkey> {
    if let Ok(pubkey) = Pubkey::from_str(claimant) {
        println!("🔍 Using provided pubkey: {}", pubkey);
        Ok(pubkey)
    } else {
        let keypair_path = PathBuf::from(claimant);
        println!("🔍 Reading keypair from: {}", keypair_path.display());
        let keypair = read_keypair_file(&keypair_path).map_err(|e| {
            CliError::InvalidConfig(format!("Failed to read keypair from '{}': {}", claimant, e))
        })?;
        let pubkey = keypair.pubkey();
        println!("🔑 Derived pubkey: {}", pubkey);
        Ok(pubkey)
    }
}

fn query_claim_receipts(
    db: &CampaignDatabase,
    client: &PrismProtocolClient,
    campaign_address: &Pubkey,
    claimant: &Pubkey,
) -> CliResult<Vec<ClaimInfo>> {
    println!("📋 Getting cohorts from database...");

    // Get all cohorts from the database
    let cohorts = db
        .read_cohorts()
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read cohorts: {}", e)))?;

    println!("✅ Found {} cohort(s) in database", cohorts.len());

    let mut claims = Vec::new();

    for cohort in cohorts {
        println!("🔍 Checking cohort: {}", cohort.name);

        // Calculate cohort address
        let (cohort_address, _) = client
            .address_finder()
            .find_cohort_v0_address(campaign_address, &cohort.merkle_root);

        // Check if claimant has a claim receipt for this cohort
        match client.get_claim_receipt_v0(&cohort_address, claimant) {
            Ok(Some(claim_receipt)) => {
                // Calculate claim receipt address for display
                let (claim_receipt_address, _) = client
                    .address_finder()
                    .find_claim_receipt_v0_address(&cohort_address, claimant);

                claims.push(ClaimInfo {
                    claim_receipt_address,
                    cohort_address,
                    cohort_name: cohort.name.clone(),
                    claimant: claim_receipt.claimant,
                    assigned_vault: claim_receipt.assigned_vault,
                    claimed_at_timestamp: claim_receipt.claimed_at_timestamp,
                });

                println!("   ✅ Found claim receipt");
            }
            Ok(None) => {
                println!("   ❌ No claim receipt found");
            }
            Err(e) => {
                println!("   ⚠️  Error checking claim receipt: {}", e);
                // Continue with other cohorts
            }
        }
    }

    Ok(claims)
}

fn display_claims_results(claims: &[ClaimInfo], campaign_fingerprint: &[u8; 32]) {
    if claims.is_empty() {
        println!("\n❌ No claims found on-chain");
        println!("   Either nothing has been claimed yet, or the claimant hasn't claimed from any cohorts.");
        return;
    }

    // Display results
    println!("\n✅ Found {} claim(s) on-chain:\n", claims.len());

    for (i, claim) in claims.iter().enumerate() {
        println!("{}. Claim Receipt: {}", i + 1, claim.claim_receipt_address);
        println!("   Cohort: {}", claim.cohort_name);
        println!("   Cohort Address: {}", claim.cohort_address);
        println!("   Claimant: {}", claim.claimant);
        println!("   Assigned Vault: {}", claim.assigned_vault);

        // Format timestamp
        let datetime = chrono::DateTime::from_timestamp(claim.claimed_at_timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Invalid timestamp".to_string());
        println!("   Claimed At: {}", datetime);
        println!();
    }

    // Summary
    println!("📊 Summary:");
    println!("   Total claim receipts: {}", claims.len());
    println!("   Campaign: {}", hex::encode(campaign_fingerprint));

    println!("\n💡 This shows what has actually been claimed on-chain.");
    println!("   Compare with 'check-eligibility' to see what's still claimable.");
}

use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_db::CampaignDatabase;
use prism_protocol_sdk::AddressFinder;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
struct ClaimInfo {
    claim_receipt_address: Pubkey,
    cohort_address: Pubkey,
    claimant: Pubkey,
    amount_claimed: u64,
    // Note: We'd need to parse the actual claim receipt account data
    // This is a simplified version - actual implementation would decode the account
}

pub fn execute(campaign_db_path: PathBuf, claimant: String, rpc_url: String) -> CliResult<()> {
    println!("⛓️  Querying blockchain for claims...");

    // Open database and RPC client
    let db = CampaignDatabase::open(&campaign_db_path)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to open database: {}", e)))?;
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

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

    // Calculate campaign address for filtering
    let address_finder = AddressFinder::default();
    let (campaign_address, _) =
        address_finder.find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);
    println!("🏛️  Campaign address: {}", campaign_address);

    // Query blockchain for claim receipts
    println!("\n🌐 Querying blockchain for claim receipts...");
    let claims = query_claim_receipts(&rpc_client, &campaign_address, &claimant_pubkey)?;

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
    _rpc_client: &RpcClient,
    _campaign_address: &Pubkey,
    _claimant: &Pubkey,
) -> CliResult<Vec<ClaimInfo>> {
    // TODO: Implement actual getProgramAccounts query
    // This would require:
    // 1. The actual Prism Protocol program ID
    // 2. Account discriminators for claim receipt accounts
    // 3. Proper account data parsing

    println!("⚠️  Note: Blockchain querying not yet implemented");
    println!("   This is a proof-of-concept placeholder");
    println!("   Real implementation would use getProgramAccounts with filters:");
    println!("   - Program: prism-protocol");
    println!("   - Account type: claim receipts");
    println!("   - Filter by campaign address");
    println!("   - Filter by claimant pubkey");

    // Return empty for now
    Ok(Vec::new())

    // Future implementation would look like:
    /*
    let program_id = Pubkey::from_str("PRISM_PROTOCOL_PROGRAM_ID")?;

    let accounts = rpc_client.get_program_accounts_with_config(
        &program_id,
        RpcProgramAccountsConfig {
            filters: Some(vec![
                // Filter for claim receipt account discriminator
                RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, CLAIM_RECEIPT_DISCRIMINATOR)),
                // Filter by claimant pubkey (offset depends on account structure)
                RpcFilterType::Memcmp(Memcmp::new_raw_bytes(8, claimant.to_bytes())),
                // Additional filtering by campaign/cohort if possible
            ]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                ..Default::default()
            },
            ..Default::default()
        },
    )?;

    // Parse account data into ClaimInfo structs
    let mut claims = Vec::new();
    for (pubkey, account) in accounts {
        // Parse account.data to extract claim information
        // This requires knowledge of the claim receipt account structure
        claims.push(ClaimInfo {
            claim_receipt_address: pubkey,
            cohort_address: todo!("parse from account data"),
            claimant: *claimant,
            amount_claimed: todo!("parse from account data"),
        });
    }

    Ok(claims)
    */
}

fn display_claims_results(claims: &[ClaimInfo], campaign_fingerprint: &[u8; 32]) {
    if claims.is_empty() {
        println!("❌ No claims found on-chain");
        println!("   Either nothing has been claimed yet, or the campaign is not deployed.");
        return;
    }

    // Display results
    println!("✅ Found {} claim(s) on-chain:\n", claims.len());

    let mut total_claimed = 0u64;

    for (i, claim) in claims.iter().enumerate() {
        println!("{}. Claim Receipt: {}", i + 1, claim.claim_receipt_address);
        println!("   Cohort Address: {}", claim.cohort_address);
        println!("   Amount Claimed: {} tokens", claim.amount_claimed);
        println!("   Claimant: {}", claim.claimant);
        total_claimed += claim.amount_claimed;
        println!();
    }

    // Summary
    println!("📊 Summary:");
    println!("   Total claim receipts: {}", claims.len());
    println!("   Total tokens claimed: {} tokens", total_claimed);
    println!("   Campaign: {}", hex::encode(campaign_fingerprint));

    println!("\n💡 This shows what has actually been claimed on-chain.");
    println!("   Compare with 'check-eligibility' to see what's still claimable.");
}

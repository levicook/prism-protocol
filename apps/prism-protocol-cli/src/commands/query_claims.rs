use crate::error::{CliError, CliResult};
use hex;
use prism_protocol_sdk::AddressFinder;
use rusqlite::Connection;
use solana_client::{
    rpc_client::RpcClient,
    // rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    // rpc_filter::{Memcmp, RpcFilterType},
};
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
struct ClaimInfo {
    claim_receipt_address: Pubkey,
    cohort_address: Pubkey,
    claimant: Pubkey,
    amount_claimed: u64,
    // Note: We'd need to parse the actual claim receipt account data
    // This is a simplified version - actual implementation would decode the account
}

pub fn execute(campaign_db_in: PathBuf, claimant: String, rpc_url: String) -> CliResult<()> {
    let address_finder = AddressFinder::default();

    // Auto-detect whether claimant is a pubkey or keypair file
    let claimant_pubkey = if let Ok(pubkey) = Pubkey::from_str(&claimant) {
        println!("🔍 Using provided pubkey: {}", pubkey);
        pubkey
    } else {
        let keypair_path = PathBuf::from(&claimant);
        println!("🔍 Reading keypair from: {}", keypair_path.display());
        let keypair = read_keypair_file(&keypair_path).map_err(|e| {
            CliError::InvalidConfig(format!("Failed to read keypair from '{}': {}", claimant, e))
        })?;
        let pubkey = keypair.pubkey();
        println!("🔑 Derived pubkey: {}", pubkey);
        pubkey
    };

    println!(
        "⛓️  Querying blockchain for actual claims by: {}",
        claimant_pubkey
    );

    // Read campaign info from database
    println!("📊 Reading campaign information...");
    let campaign_info = read_campaign_info(&campaign_db_in)?;
    println!("✅ Campaign: {}", hex::encode(campaign_info.fingerprint));
    println!("   Mint: {}", campaign_info.mint);
    println!("   Admin: {}", campaign_info.admin);

    // Calculate campaign address for filtering
    let (campaign_address, _) = address_finder.find_campaign_v0_address(&campaign_info.admin, &campaign_info.fingerprint);
    println!("🏛️  Campaign address: {}", campaign_address);

    // Query blockchain for claim receipts
    println!("\n🌐 Querying blockchain for claim receipts...");
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Note: This is a simplified implementation
    // In reality, we'd need the actual program ID and account discriminators
    // For now, let's simulate this as a proof of concept

    let claims = query_claim_receipts(&rpc_client, &campaign_address, &claimant_pubkey)?;

    if claims.is_empty() {
        println!(
            "❌ No claims found on-chain for claimant {}",
            claimant_pubkey
        );
        println!("   Either nothing has been claimed yet, or the campaign is not deployed.");
        return Ok(());
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
    println!("   Campaign: {}", hex::encode(campaign_info.fingerprint));

    println!("\n💡 This shows what has actually been claimed on-chain.");
    println!("   Compare with 'check-eligibility' to see what's still claimable.");

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
            // ... parse other fields from account data
        });
    }

    Ok(claims)
    */
}

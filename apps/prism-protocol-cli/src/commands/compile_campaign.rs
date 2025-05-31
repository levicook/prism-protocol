use crate::error::{CliError, CliResult};
use prism_protocol_client::PrismProtocolClient;
use prism_protocol_sdk::{compile_campaign, AddressFinder};
use rust_decimal::Decimal;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
};
use std::str::FromStr;
use std::{path::PathBuf, sync::Arc};

pub fn execute(
    campaign_csv_in: PathBuf,
    cohorts_csv_in: PathBuf,
    mint: Pubkey,
    budget: String,
    admin_keypair: PathBuf,
    claimants_per_vault: usize,
    campaign_db_out: PathBuf,
    rpc_url: String,
) -> CliResult<()> {
    println!("Starting campaign compilation");
    println!("Campaign CSV: {}", campaign_csv_in.display());
    println!("Cohorts CSV: {}", cohorts_csv_in.display());
    println!("Mint: {}", mint);
    println!("Budget: {}", budget);
    println!("Admin keypair: {}", admin_keypair.display());
    println!("Claimants per vault: {}", claimants_per_vault);
    println!("Output database: {}", campaign_db_out.display());
    println!("RPC URL: {}", rpc_url);

    // Parse budget
    println!("Parsing budget...");
    let budget_decimal = Decimal::from_str(&budget).map_err(|e| {
        CliError::InvalidConfig(format!("Invalid budget format '{}': {}", budget, e))
    })?;
    println!("Budget parsed as: {}", budget_decimal);

    // Read and validate admin keypair
    println!("Reading admin keypair...");
    let admin_keypair = read_keypair_file(&admin_keypair)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to read admin keypair: {}", e)))?;
    let admin_pubkey = admin_keypair.pubkey();
    println!("Admin public key: {}", admin_pubkey);

    // Discover mint decimals using our custom RPC client
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        &rpc_url,
        CommitmentConfig::confirmed(),
    ));

    let client = PrismProtocolClient::new(rpc_client.clone());

    let mint_info = client
        .get_mint(&mint)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to fetch mint {}: {}", mint, e)))?
        .ok_or_else(|| CliError::InvalidConfig(format!("Mint {} not found", mint)))?;

    let mint_decimals = mint_info.decimals;
    println!("Discovered mint decimals: {}", mint_decimals);

    // Check if output file exists
    if campaign_db_out.exists() {
        println!(
            "Output database file already exists and will be overwritten: {}",
            campaign_db_out.display()
        );
    }

    // Use SDK to compile campaign
    println!("Compiling campaign from CSV files...");
    let address_finder = AddressFinder::default();

    let db = compile_campaign(
        address_finder,
        &campaign_csv_in,
        &cohorts_csv_in,
        budget_decimal,
        mint,
        mint_decimals,
        admin_pubkey,
        claimants_per_vault,
    )
    .map_err(|e| CliError::InvalidConfig(format!("Campaign compilation failed: {}", e)))?;

    // Save database to file
    println!("Saving compiled campaign to database file...");
    db.save_to_file(&campaign_db_out, true)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to save database: {}", e)))?;

    println!("Campaign compilation completed successfully!");
    println!("Database saved to: {}", campaign_db_out.display());

    Ok(())
}

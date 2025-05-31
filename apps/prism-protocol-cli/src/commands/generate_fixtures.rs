use crate::error::{CliError, CliResult};
use csv::Writer;
use rand::Rng;
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    signature::Signer,
    signer::keypair::{write_keypair_file, Keypair},
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::{
    instruction::{initialize_mint2, mint_to},
    state::Mint,
};
use std::{fs, path::PathBuf, str::FromStr, sync::Arc, thread, time::Duration};

/// Generate test fixtures with organized directory structure and real keypairs
pub fn execute(
    campaign_name: String,
    output_dir: PathBuf,
    claimant_count: u64,
    distribution: String,
    min_entitlements: u64,
    max_entitlements: u64,
    cohort_count: usize,
    mint_decimals: u8,
    budget: String,
    rpc_url: String,
) -> CliResult<()> {
    println!("🚀 Enhanced Fixture Generation with Mint Creation");
    println!("Campaign: {}", campaign_name);
    println!("Generating {} claimants with real keypairs", claimant_count);
    println!("Distribution: {}", distribution);
    println!("Cohorts: {}", cohort_count);
    println!("Mint decimals: {}", mint_decimals);
    println!("Budget: {} tokens", budget);

    // Parse budget as Decimal for precision
    let budget_decimal = Decimal::from_str(&budget)
        .map_err(|e| CliError::InvalidConfig(format!("Invalid budget '{}': {}", budget, e)))?;

    // Create slugified directory name
    let campaign_slug = slugify(&campaign_name);
    let fixture_dir = output_dir.join(&campaign_slug);

    // Check if directory already exists
    if fixture_dir.exists() {
        return Err(CliError::InvalidConfig(format!(
            "Fixture directory already exists: {}\n\nTo regenerate fixtures, first remove the existing directory:\n  rm -rf {}\n\nOr use a different campaign name.",
            fixture_dir.display(),
            fixture_dir.display()
        )));
    }

    // Create directory structure
    fs::create_dir_all(&fixture_dir)?;
    let keypairs_dir = fixture_dir.join("claimant-keypairs");
    fs::create_dir_all(&keypairs_dir)?;

    println!("📁 Created fixture directory: {}", fixture_dir.display());

    // Generate admin keypair for mint authority
    println!("\n🔑 Generating admin keypair...");
    let admin_keypair = Keypair::new();
    let admin_keypair_path = fixture_dir.join("admin.json");
    write_keypair_file(&admin_keypair, &admin_keypair_path)
        .map_err(|e| CliError::WriteKeypair(e.to_string()))?;
    println!("✅ Admin keypair saved: {}", admin_keypair_path.display());
    println!("   Admin pubkey: {}", admin_keypair.pubkey());

    // Generate mint keypair
    println!("\n🪙 Generating test mint keypair...");
    let mint_keypair = Keypair::new();
    let mint_keypair_path = fixture_dir.join("mint.json");
    write_keypair_file(&mint_keypair, &mint_keypair_path)
        .map_err(|e| CliError::WriteKeypair(e.to_string()))?;
    println!("✅ Mint keypair saved: {}", mint_keypair_path.display());
    println!("   Mint pubkey: {}", mint_keypair.pubkey());

    // Create RPC client and Prism Protocol client
    println!("\n🌐 Connecting to Solana cluster...");

    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        &rpc_url,
        CommitmentConfig::confirmed(),
    ));

    println!("✅ Connected to: {}", rpc_url);

    // Fund admin with airdrop (fail fast for simplicity)
    println!("\n💰 Funding admin account...");
    let airdrop_amount = 10 * LAMPORTS_PER_SOL; // 10 SOL
    match rpc_client.request_airdrop(&admin_keypair.pubkey(), airdrop_amount) {
        Ok(signature) => {
            println!("✅ Airdrop requested: {}", signature);
            println!("   Waiting 5 seconds for confirmation...");
            thread::sleep(Duration::from_secs(5));

            match rpc_client.get_balance(&admin_keypair.pubkey()) {
                Ok(balance) => {
                    println!(
                        "✅ Admin funded with {} SOL",
                        balance as f64 / LAMPORTS_PER_SOL as f64
                    );
                }
                Err(e) => {
                    return Err(CliError::InvalidConfig(format!(
                        "Failed to confirm airdrop: {}",
                        e
                    )));
                }
            }
        }
        Err(e) => {
            return Err(CliError::InvalidConfig(format!(
                "Airdrop failed (ensure cluster supports airdrops): {}",
                e
            )));
        }
    }

    // Create mint account on-chain
    println!("\n🏗️  Creating mint account on-chain...");
    let mint_rent = rpc_client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .map_err(|e| CliError::InvalidConfig(format!("Failed to get mint rent: {}", e)))?;

    let create_mint_account_ix = system_instruction::create_account(
        &admin_keypair.pubkey(),
        &mint_keypair.pubkey(),
        mint_rent,
        Mint::LEN as u64,
        &spl_token::ID,
    );

    let initialize_mint_ix = initialize_mint2(
        &spl_token::ID,
        &mint_keypair.pubkey(),
        &admin_keypair.pubkey(),       // mint authority
        Some(&admin_keypair.pubkey()), // freeze authority
        mint_decimals,
    )
    .map_err(|e| {
        CliError::InvalidConfig(format!(
            "Failed to create initialize mint instruction: {}",
            e
        ))
    })?;

    let transaction = Transaction::new_signed_with_payer(
        &[create_mint_account_ix, initialize_mint_ix],
        Some(&admin_keypair.pubkey()),
        &[&admin_keypair, &mint_keypair],
        rpc_client
            .get_latest_blockhash()
            .map_err(|e| CliError::InvalidConfig(format!("Failed to get blockhash: {}", e)))?,
    );

    match rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
        Ok(signature) => {
            println!("✅ Mint created on-chain: {}", signature);
            println!("   Mint decimals: {}", mint_decimals);
            println!("   Mint authority: {}", admin_keypair.pubkey());
        }
        Err(e) => {
            return Err(CliError::InvalidConfig(format!(
                "Failed to create mint: {}",
                e
            )));
        }
    }

    // Create admin's associated token account and mint budget tokens
    println!("\n💸 Minting budget tokens for admin...");
    let admin_ata = get_associated_token_address(&admin_keypair.pubkey(), &mint_keypair.pubkey());

    // Convert budget to base units (multiply by 10^decimals)
    let base_unit_multiplier = 10u64.pow(mint_decimals as u32);
    let budget_base_units = budget_decimal
        .checked_mul(Decimal::from(base_unit_multiplier))
        .and_then(|d| d.floor().to_u64())
        .ok_or_else(|| {
            CliError::InvalidConfig(format!(
                "Budget overflow: {} tokens with {} decimals exceeds u64 range",
                budget, mint_decimals
            ))
        })?;

    let create_ata_ix = create_associated_token_account(
        &admin_keypair.pubkey(), // payer
        &admin_keypair.pubkey(), // owner
        &mint_keypair.pubkey(),  // mint
        &spl_token::ID,
    );

    let mint_to_ix = mint_to(
        &spl_token::ID,
        &mint_keypair.pubkey(),  // mint
        &admin_ata,              // destination
        &admin_keypair.pubkey(), // mint authority
        &[],                     // signers
        budget_base_units,       // amount
    )
    .map_err(|e| CliError::InvalidConfig(format!("Failed to create mint_to instruction: {}", e)))?;

    let mint_transaction = Transaction::new_signed_with_payer(
        &[create_ata_ix, mint_to_ix],
        Some(&admin_keypair.pubkey()),
        &[&admin_keypair],
        rpc_client.get_latest_blockhash().map_err(|e| {
            CliError::InvalidConfig(format!("Failed to get blockhash for minting: {}", e))
        })?,
    );

    match rpc_client.send_and_confirm_transaction_with_spinner(&mint_transaction) {
        Ok(signature) => {
            println!("✅ Minted {} tokens for admin: {}", budget, signature);
            println!("   Admin ATA: {}", admin_ata);
            println!("   Amount (base units): {}", budget_base_units);
        }
        Err(e) => {
            return Err(CliError::InvalidConfig(format!(
                "Failed to mint tokens: {}",
                e
            )));
        }
    }

    // Generate cohort names
    let cohort_names = generate_cohort_names(cohort_count);

    // Generate cohorts CSV with share percentages that sum to 100%
    let cohorts_csv_path = fixture_dir.join("cohorts.csv");
    println!("\n📋 Generating cohorts configuration with share percentages...");
    generate_cohorts_csv_with_percentages(&cohorts_csv_path, &cohort_names)?;
    println!(
        "✅ Generated cohorts config: {}",
        cohorts_csv_path.display()
    );

    // Generate campaign CSV with real keypairs
    let campaign_csv_path = fixture_dir.join("campaign.csv");
    println!("\n📋 Generating campaign claimants with real keypairs...");
    let claimant_data = generate_campaign_csv_with_keypairs(
        &campaign_csv_path,
        &cohort_names,
        claimant_count,
        &distribution,
        min_entitlements,
        max_entitlements,
    )?;
    println!(
        "✅ Generated campaign claimants: {}",
        campaign_csv_path.display()
    );

    // Save individual keypair files
    println!("\n🔑 Saving individual keypair files...");
    save_claimant_keypairs(&keypairs_dir, &claimant_data)?;
    println!("✅ Saved {} keypair files", claimant_data.len());

    // Generate example compile command
    println!("\n📝 Example compile command:");
    println!("cargo run -p prism-protocol-cli -- compile-campaign \\");
    println!("  --campaign-csv-in {} \\", campaign_csv_path.display());
    println!("  --cohorts-csv-in {} \\", cohorts_csv_path.display());
    println!("  --mint {} \\", mint_keypair.pubkey());
    println!("  --budget \"{}\" \\", budget);
    println!("  --admin-keypair {} \\", admin_keypair_path.display());
    println!("  --campaign-db-out {}/campaign.db", fixture_dir.display());

    println!("\n🎉 Enhanced fixture generation completed!");
    println!("📊 Summary:");
    println!("  - Campaign: {} ({})", campaign_name, campaign_slug);
    println!("  - {} claimants with real keypairs", claimant_count);
    println!(
        "  - {} cohorts with percentage-based allocation",
        cohort_count
    );
    println!("  - Distribution: {}", distribution);
    println!(
        "  - Mint: {} ({} decimals)",
        mint_keypair.pubkey(),
        mint_decimals
    );
    println!(
        "  - Budget: {} tokens ({} base units)",
        budget, budget_base_units
    );
    println!("  - Admin keypair: {}", admin_keypair_path.display());
    println!("  - Mint keypair: {}", mint_keypair_path.display());
    println!("  - Directory: {}", fixture_dir.display());

    Ok(())
}

/// Convert campaign name to URL-safe slug
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Data structure for claimant information with keypair
#[derive(Debug)]
#[allow(dead_code)]
struct ClaimantData {
    index: u64,
    keypair: Keypair,
    cohort: String,
    entitlements: u64,
}

/// Generate campaign CSV with real keypairs and return claimant data
fn generate_campaign_csv_with_keypairs(
    path: &PathBuf,
    cohort_names: &[String],
    claimant_count: u64,
    distribution: &str,
    min_entitlements: u64,
    max_entitlements: u64,
) -> CliResult<Vec<ClaimantData>> {
    let mut writer = Writer::from_path(path)?;
    writer.write_record(&["cohort", "claimant", "entitlements"])?;

    let mut claimant_data = Vec::new();

    for i in 0..claimant_count {
        // Generate real random keypair (deterministic if seed provided)
        let keypair = Keypair::new();

        // Assign to cohort (distribute evenly across cohorts)
        let cohort_index = (i as usize) % cohort_names.len();
        let cohort_name = &cohort_names[cohort_index];

        // Generate entitlements based on distribution
        let entitlements = match distribution {
            "uniform" => {
                let mut rng = rand::thread_rng();
                rng.gen_range(min_entitlements..=max_entitlements)
            }
            "realistic" => generate_realistic_entitlements(min_entitlements, max_entitlements),
            "exponential" => generate_exponential_entitlements(min_entitlements, max_entitlements),
            _ => {
                return Err(CliError::InvalidConfig(format!(
                    "Unknown distribution type: {}. Valid options: uniform, realistic, exponential",
                    distribution
                )));
            }
        };

        // Write to CSV
        writer.write_record(&[
            cohort_name,
            &keypair.pubkey().to_string(),
            &entitlements.to_string(),
        ])?;

        // Store claimant data for keypair file generation
        claimant_data.push(ClaimantData {
            index: i,
            keypair,
            cohort: cohort_name.clone(),
            entitlements,
        });

        // Progress indicator for large datasets
        if claimant_count > 10_000 && i % 10_000 == 0 {
            println!("Generated {} / {} claimants", i, claimant_count);
        }
    }

    writer.flush()?;
    Ok(claimant_data)
}

/// Save individual keypair files
fn save_claimant_keypairs(keypairs_dir: &PathBuf, claimant_data: &[ClaimantData]) -> CliResult<()> {
    for data in claimant_data {
        let filename = format!("claimant-{:04}.json", data.index + 1);
        let keypair_path = keypairs_dir.join(&filename);

        // Write keypair file
        write_keypair_file(&data.keypair, &keypair_path)
            .map_err(|e| CliError::WriteKeypair(e.to_string()))?;

        // Progress for large datasets
        if claimant_data.len() > 10_000 && (data.index + 1) % 10_000 == 0 {
            println!(
                "Saved {} / {} keypair files",
                data.index + 1,
                claimant_data.len()
            );
        }
    }

    Ok(())
}

fn generate_cohort_names(count: usize) -> Vec<String> {
    let base_names = vec![
        "early_supporters",
        "community_rewards",
        "team_allocation",
        "advisors",
        "partners",
        "ecosystem_fund",
        "liquidity_providers",
        "validators",
        "developers",
        "content_creators",
        "beta_testers",
        "ambassadors",
    ];

    let mut names = Vec::new();
    for i in 0..count {
        if i < base_names.len() {
            names.push(base_names[i].to_string());
        } else {
            names.push(format!("cohort_{}", i + 1));
        }
    }
    names
}

fn generate_cohorts_csv_with_percentages(path: &PathBuf, cohort_names: &[String]) -> CliResult<()> {
    let mut writer = Writer::from_path(path)?;
    writer.write_record(&["cohort", "share_percentage"])?;

    // Generate random percentages that sum to exactly 100%
    let mut rng = rand::thread_rng();
    let cohort_count = cohort_names.len();

    // Generate random weights using Decimal
    let weights: Vec<Decimal> = (0..cohort_count)
        .map(|_| {
            let random_weight = rng.gen_range(1.0..10.0);
            Decimal::from_f64(random_weight).unwrap_or(Decimal::ONE)
        })
        .collect();

    // Normalize to sum to exactly 100%
    let total_weight: Decimal = weights.iter().sum();
    let hundred = Decimal::new(100, 0); // 100.0

    let mut percentages: Vec<Decimal> = weights
        .iter()
        .map(|w| (w / total_weight) * hundred)
        .collect();

    // Ensure exact sum to 100% by adjusting the last cohort
    let last_index = percentages.len() - 1;
    let current_sum: Decimal = percentages[..last_index].iter().sum();
    percentages[last_index] = hundred - current_sum;

    // Write to CSV with proper formatting
    for (cohort_name, percentage) in cohort_names.iter().zip(percentages.iter()) {
        writer.write_record(&[
            cohort_name,
            &percentage.to_string(), // Decimal provides precise string representation
        ])?;
    }

    writer.flush()?;

    // Verify sum for debugging
    let sum: Decimal = percentages.iter().sum();
    println!("   Share percentages sum: {}%", sum);

    Ok(())
}

/// Generate realistic entitlements (weighted towards lower values)
fn generate_realistic_entitlements(min: u64, max: u64) -> u64 {
    let range = max - min + 1;
    let mut rng = rand::thread_rng();

    // Use inverse exponential to weight towards lower values
    let random_val: f64 = rng.gen();
    let weighted = 1.0 - (-random_val * 2.0).exp(); // Exponential decay

    min + (weighted * range as f64) as u64
}

/// Generate exponential distribution entitlements
fn generate_exponential_entitlements(min: u64, max: u64) -> u64 {
    let range = max - min + 1;
    let mut rng = rand::thread_rng();

    // Exponential distribution with lambda = 2
    let random_val: f64 = rng.gen();
    let exponential = (-random_val.ln() / 2.0).min(1.0); // Cap at 1.0

    min + (exponential * range as f64) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_keypair_uniqueness() {
        let mut pubkeys = HashSet::new();

        // Generate 100 keypairs and ensure their pubkeys are all unique
        for i in 0..100 {
            let keypair = Keypair::new();
            assert!(
                pubkeys.insert(keypair.pubkey()),
                "Duplicate pubkey found at index {}",
                i
            );
        }
    }

    #[test]
    fn test_entitlements_in_range() {
        let min = 10;
        let max = 100;

        for _ in 0..1000 {
            let entitlements = generate_realistic_entitlements(min, max);
            assert!(entitlements >= min && entitlements <= max);
        }
    }

    #[test]
    fn test_cohort_name_generation() {
        let names = generate_cohort_names(3);
        assert_eq!(
            names,
            vec!["early_supporters", "community_rewards", "team_allocation"]
        );

        let names = generate_cohort_names(15);
        assert_eq!(names.len(), 15);
        assert!(names[14].starts_with("cohort_"));
    }
}

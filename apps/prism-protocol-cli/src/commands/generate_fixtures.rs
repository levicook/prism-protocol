use crate::error::{CliError, CliResult};
use csv::Writer;
use rand::Rng;
use solana_sdk::{
    signature::Signer,
    signer::keypair::{write_keypair_file, Keypair},
};
use std::{fs, path::PathBuf};

/// Generate test fixtures with organized directory structure and real keypairs
pub fn execute(
    campaign_name: String,
    output_dir: PathBuf,
    count: u64,
    distribution: String,
    min_entitlements: u64,
    max_entitlements: u64,
    cohort_count: usize,
    min_amount_per_entitlement: u64,
    max_amount_per_entitlement: u64,
) -> CliResult<()> {
    println!("🚀 Enhanced Fixture Generation");
    println!("Campaign: {}", campaign_name);
    println!("Generating {} claimants with real keypairs", count);
    println!("Distribution: {}", distribution);
    println!("Cohorts: {}", cohort_count);

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

    // Generate cohort names
    let cohort_names = generate_cohort_names(cohort_count);

    // Generate cohorts CSV
    let cohorts_csv_path = fixture_dir.join("cohorts.csv");
    println!("\n📋 Generating cohorts configuration...");
    generate_cohorts_csv(
        &cohorts_csv_path,
        &cohort_names,
        min_amount_per_entitlement,
        max_amount_per_entitlement,
    )?;
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
        count,
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

    println!("\n🎉 Enhanced fixture generation completed!");
    println!("📊 Summary:");
    println!("  - Campaign: {} ({})", campaign_name, campaign_slug);
    println!("  - {} claimants with real keypairs", count);
    println!("  - {} cohorts", cohort_count);
    println!("  - Distribution: {}", distribution);
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
    count: u64,
    distribution: &str,
    min_entitlements: u64,
    max_entitlements: u64,
) -> CliResult<Vec<ClaimantData>> {
    let mut writer = Writer::from_path(path)?;
    writer.write_record(&["cohort", "claimant", "entitlements"])?;

    let mut claimant_data = Vec::new();

    for i in 0..count {
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
        if count > 10_000 && i % 10_000 == 0 {
            println!("Generated {} / {} claimants", i, count);
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

fn generate_cohorts_csv(
    path: &PathBuf,
    cohort_names: &[String],
    min_amount: u64,
    max_amount: u64,
) -> CliResult<()> {
    let mut writer = Writer::from_path(path)?;
    writer.write_record(&["cohort", "amount_per_entitlement"])?;

    let mut rng = rand::thread_rng();

    for cohort_name in cohort_names {
        let amount_per_entitlement = rng.gen_range(min_amount..=max_amount);

        writer.write_record(&[cohort_name, &amount_per_entitlement.to_string()])?;
    }

    writer.flush()?;
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

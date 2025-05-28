use clap::{Parser, Subcommand};
use solana_sdk::pubkey::Pubkey;
use std::path::PathBuf;

mod commands;
mod config;
mod error;

use error::CliResult;

#[derive(Parser)]
#[command(name = "prism-protocol")]
#[command(about = "Prism Protocol CLI - Efficient token distribution on Solana")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate test fixtures for benchmarking
    GenerateFixtures {
        /// Number of claimants to generate
        #[arg(long)]
        count: u64,

        /// Seed for deterministic generation
        #[arg(long, default_value = "42")]
        seed: u64,

        /// Output campaign CSV file path
        #[arg(long, default_value = "campaign.csv")]
        campaign_csv_out: PathBuf,

        /// Output cohorts CSV file path
        #[arg(long, default_value = "cohorts.csv")]
        cohorts_csv_out: PathBuf,

        /// Distribution type
        #[arg(long, default_value = "uniform")]
        distribution: String,

        /// Minimum entitlements per claimant
        #[arg(long, default_value = "1")]
        min_entitlements: u64,

        /// Maximum entitlements per claimant
        #[arg(long, default_value = "100")]
        max_entitlements: u64,

        /// Number of cohorts to generate
        #[arg(long, default_value = "3")]
        cohort_count: usize,

        /// Minimum amount per entitlement (in token base units)
        #[arg(long, default_value = "1000000")]
        min_amount_per_entitlement: u64,

        /// Maximum amount per entitlement (in token base units)
        #[arg(long, default_value = "10000000")]
        max_amount_per_entitlement: u64,
    },

    /// Generate campaign data from configuration
    GenerateCampaign {
        /// Input campaign claimants file (cohort,claimant,entitlements)
        #[arg(long)]
        campaign_csv_in: PathBuf,

        /// Input cohort configuration file (cohort,amount_per_entitlement)
        #[arg(long)]
        cohorts_csv_in: PathBuf,

        /// SPL token mint to distribute
        #[arg(short, long)]
        mint: Pubkey,

        /// Admin keypair file
        #[arg(long)]
        admin_keypair: PathBuf,

        /// Target claimants per vault (for vault count calculation)
        #[arg(long, default_value = "200000")]
        claimants_per_vault: usize,

        /// Output SQLite database file
        #[arg(long, default_value = "campaign.db")]
        campaign_db_out: PathBuf,
    },

    /// Deploy campaign on-chain
    DeployCampaign {
        /// Campaign configuration file
        #[arg(short, long)]
        config: PathBuf,

        /// Admin keypair file
        #[arg(short, long)]
        keypair: PathBuf,

        /// Solana RPC URL
        #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
        rpc_url: String,
    },

    /// Deploy a specific cohort on-chain
    DeployCohort {
        /// Campaign fingerprint (hex string)
        #[arg(short, long)]
        campaign: String,

        /// Cohort merkle root (hex string)
        #[arg(short, long)]
        merkle_root: String,

        /// Admin keypair file
        #[arg(short, long)]
        keypair: PathBuf,

        /// Solana RPC URL
        #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
        rpc_url: String,
    },

    /// Pause a campaign
    PauseCampaign {
        /// Campaign fingerprint (hex string)
        campaign: String,

        /// Admin keypair file
        #[arg(short, long)]
        keypair: PathBuf,

        /// Solana RPC URL
        #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
        rpc_url: String,
    },

    /// Resume a campaign
    ResumeCampaign {
        /// Campaign fingerprint (hex string)
        campaign: String,

        /// Admin keypair file
        #[arg(short, long)]
        keypair: PathBuf,

        /// Solana RPC URL
        #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
        rpc_url: String,
    },

    /// Reclaim tokens from a cohort
    ReclaimTokens {
        /// Campaign fingerprint (hex string)
        campaign: String,

        /// Cohort merkle root (hex string)
        cohort: String,

        /// Admin keypair file
        #[arg(short, long)]
        keypair: PathBuf,

        /// Solana RPC URL
        #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
        rpc_url: String,
    },

    /// Get campaign status
    CampaignStatus {
        /// Campaign fingerprint (hex string)
        campaign: String,

        /// Solana RPC URL
        #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
        rpc_url: String,
    },
}

fn main() -> CliResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateFixtures {
            count,
            seed,
            campaign_csv_out,
            cohorts_csv_out,
            distribution,
            min_entitlements,
            max_entitlements,
            cohort_count,
            min_amount_per_entitlement,
            max_amount_per_entitlement,
        } => commands::generate_fixtures::execute(
            count,
            seed,
            campaign_csv_out,
            cohorts_csv_out,
            distribution,
            min_entitlements,
            max_entitlements,
            cohort_count,
            min_amount_per_entitlement,
            max_amount_per_entitlement,
        ),

        Commands::GenerateCampaign {
            campaign_csv_in,
            cohorts_csv_in,
            mint,
            admin_keypair,
            claimants_per_vault,
            campaign_db_out,
        } => commands::generate_campaign::execute(
            campaign_csv_in,
            cohorts_csv_in,
            mint,
            admin_keypair,
            claimants_per_vault,
            campaign_db_out,
        ),

        Commands::DeployCampaign {
            config,
            keypair,
            rpc_url,
        } => commands::deploy_campaign::execute(config, keypair, rpc_url),

        Commands::DeployCohort {
            campaign,
            merkle_root,
            keypair,
            rpc_url,
        } => commands::deploy_cohort::execute(campaign, merkle_root, keypair, rpc_url),

        Commands::PauseCampaign {
            campaign,
            keypair,
            rpc_url,
        } => commands::pause_campaign::execute(campaign, keypair, rpc_url),

        Commands::ResumeCampaign {
            campaign,
            keypair,
            rpc_url,
        } => commands::resume_campaign::execute(campaign, keypair, rpc_url),

        Commands::ReclaimTokens {
            campaign,
            cohort,
            keypair,
            rpc_url,
        } => commands::reclaim_tokens::execute(campaign, cohort, keypair, rpc_url),

        Commands::CampaignStatus { campaign, rpc_url } => {
            commands::campaign_status::execute(campaign, rpc_url)
        }
    }
}

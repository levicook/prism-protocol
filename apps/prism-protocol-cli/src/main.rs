use clap::{Parser, Subcommand};
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
        #[arg(short, long)]
        count: u64,
        
        /// Seed for deterministic generation
        #[arg(short, long, default_value = "42")]
        seed: u64,
        
        /// Output file path
        #[arg(short, long, default_value = "fixtures.csv")]
        output: PathBuf,
        
        /// Distribution type
        #[arg(short, long, default_value = "uniform")]
        distribution: String,
        
        /// Minimum entitlements per claimant
        #[arg(long, default_value = "1")]
        min_entitlements: u64,
        
        /// Maximum entitlements per claimant
        #[arg(long, default_value = "100")]
        max_entitlements: u64,
    },
    
    /// Generate campaign data from configuration
    GenerateCampaign {
        /// Campaign configuration file
        config: PathBuf,
        
        /// Output directory for generated files
        #[arg(short, long, default_value = "output")]
        output_dir: PathBuf,
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
            output, 
            distribution, 
            min_entitlements, 
            max_entitlements 
        } => {
            commands::generate_fixtures::execute(
                count,
                seed,
                output,
                distribution,
                min_entitlements,
                max_entitlements,
            )
        }
        
        Commands::GenerateCampaign { config, output_dir } => {
            commands::generate_campaign::execute(config, output_dir)
        }
        
        Commands::DeployCampaign { config, keypair, rpc_url } => {
            commands::deploy_campaign::execute(config, keypair, rpc_url)
        }
        
        Commands::DeployCohort { campaign, merkle_root, keypair, rpc_url } => {
            commands::deploy_cohort::execute(campaign, merkle_root, keypair, rpc_url)
        }
        
        Commands::PauseCampaign { campaign, keypair, rpc_url } => {
            commands::pause_campaign::execute(campaign, keypair, rpc_url)
        }
        
        Commands::ResumeCampaign { campaign, keypair, rpc_url } => {
            commands::resume_campaign::execute(campaign, keypair, rpc_url)
        }
        
        Commands::ReclaimTokens { campaign, cohort, keypair, rpc_url } => {
            commands::reclaim_tokens::execute(campaign, cohort, keypair, rpc_url)
        }
        
        Commands::CampaignStatus { campaign, rpc_url } => {
            commands::campaign_status::execute(campaign, rpc_url)
        }
    }
}

use crate::error::CliResult;
use std::path::PathBuf;

pub fn execute(
    campaign: String,
    cohort: String,
    keypair: PathBuf,
    rpc_url: String,
) -> CliResult<()> {
    println!("🚧 reclaim-tokens command not yet implemented");
    println!("Campaign: {}", campaign);
    println!("Cohort: {}", cohort);
    println!("Keypair: {}", keypair.display());
    println!("RPC URL: {}", rpc_url);
    Ok(())
}

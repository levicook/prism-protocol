use crate::error::CliResult;
use std::path::PathBuf;

pub fn execute(
    campaign: String,
    merkle_root: String,
    keypair: PathBuf,
    rpc_url: String,
) -> CliResult<()> {
    println!("🚧 deploy-cohort command not yet implemented");
    println!("Campaign: {}", campaign);
    println!("Merkle root: {}", merkle_root);
    println!("Keypair: {}", keypair.display());
    println!("RPC URL: {}", rpc_url);
    Ok(())
}

use crate::error::CliResult;

pub fn execute(campaign: String, rpc_url: String) -> CliResult<()> {
    println!("🚧 campaign-status command not yet implemented");
    println!("Campaign: {}", campaign);
    println!("RPC URL: {}", rpc_url);
    Ok(())
} 
use antikhitera_mcp_client::{Cli, RunMode};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cli = Cli::parse();
    cli.mode = Some(RunMode::Stdio);
    antikhitera_mcp_client::run(cli).await
}

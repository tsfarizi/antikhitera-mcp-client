#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use antikhitera_mcp_client::Cli;
    use clap::Parser;

    let cli = Cli::parse();
    antikhitera_mcp_client::run(cli).await
}

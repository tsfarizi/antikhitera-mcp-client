#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use antikythera_cli::{run, Cli};
    use clap::Parser;

    let cli = Cli::parse();
    run(cli).await
}

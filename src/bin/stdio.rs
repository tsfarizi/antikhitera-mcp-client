//! STDIO-only binary entry point
//!
//! This binary runs only the interactive chat interface without REST or setup menus.
//! Optimized for production deployment and CLI usage.

use antikhitera_mcp_client::application::client::{ClientConfig, McpClient};
use antikhitera_mcp_client::config::AppConfig;
use antikhitera_mcp_client::infrastructure::model::DynamicModelProvider;
use antikhitera_mcp_client::tui::screens::run_chat;
use clap::Parser;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "mcp-chat", about = "MCP Client Interactive Chat")]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let config_path = args.config.as_deref().map(Path::new);
    let file_config = AppConfig::load(config_path)?;

    let provider = DynamicModelProvider::from_configs(&file_config.providers)?;
    let client_config = ClientConfig::new(
        file_config.default_provider.clone(),
        file_config.model.clone(),
    )
    .with_tools(file_config.tools.clone())
    .with_servers(file_config.servers.clone())
    .with_prompts(file_config.prompts.clone())
    .with_providers(file_config.providers.clone());

    let client = Arc::new(McpClient::new(provider, client_config));

    let provider_name = file_config.default_provider.clone();
    let model_name = file_config.model.clone();

    match run_chat(client, provider_name, model_name).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Chat error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

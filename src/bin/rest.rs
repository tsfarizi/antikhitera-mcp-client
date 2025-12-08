//! REST-only binary entry point
//!
//! This binary runs only the REST API server without TUI or interactive features.
//! Optimized for production deployment.

use antikhitera_mcp_client::application::client::{ClientConfig, McpClient};
use antikhitera_mcp_client::config::{AppConfig, CONFIG_PATH};
use antikhitera_mcp_client::infrastructure::model::DynamicModelProvider;
use antikhitera_mcp_client::infrastructure::server;
use clap::Parser;
use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser)]
#[command(name = "mcp-rest", about = "MCP Client REST API Server")]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// REST API bind address
    #[arg(long, default_value = "127.0.0.1:8080")]
    addr: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    init_tracing();
    info!("Starting MCP REST API Server");

    let config_path = args.config.as_deref().map(Path::new);
    let file_config = AppConfig::load(config_path)?;

    debug!(provider = %file_config.default_provider, model = %file_config.model, "Configuration loaded");

    let provider = DynamicModelProvider::from_configs(&file_config.providers)?;
    let client_config = ClientConfig::new(
        file_config.default_provider.clone(),
        file_config.model.clone(),
    )
    .with_tools(file_config.tools.clone())
    .with_servers(file_config.servers.clone())
    .with_prompt_template(Some(file_config.prompt_template.clone()))
    .with_providers(file_config.providers.clone());

    let client = Arc::new(McpClient::new(provider, client_config));

    info!(addr = %args.addr, "REST server starting");
    server::serve(client, args.addr).await?;

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .init();
}

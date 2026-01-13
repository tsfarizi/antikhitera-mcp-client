//! REST-only binary entry point
//!
//! This binary runs only the REST API server without TUI or interactive features.
//! Optimized for production deployment.

use antikhitera_mcp_client::application::client::{ClientConfig, McpClient};
use antikhitera_mcp_client::application::tooling::{
    HttpTransport, HttpTransportConfig, McpTransport,
};
use antikhitera_mcp_client::config::{AppConfig, TransportType};
use antikhitera_mcp_client::infrastructure::model::DynamicModelProvider;
use antikhitera_mcp_client::infrastructure::server;
use clap::Parser;
use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser)]
#[command(name = "mcp-rest", about = "MCP Client REST API Server")]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// REST API bind address (overrides config if specified)
    #[arg(long)]
    addr: Option<SocketAddr>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    init_tracing();
    info!("Starting MCP REST API Server");

    let config_path = args.config.as_deref().map(Path::new);
    let file_config = AppConfig::load(config_path)?;

    debug!(provider = %file_config.default_provider, model = %file_config.model, "Configuration loaded");

    // Use CLI addr if provided, otherwise use config bind address
    let addr: SocketAddr = args.addr.unwrap_or_else(|| {
        file_config
            .rest_server
            .bind
            .parse()
            .expect("Invalid bind address in config")
    });

    let provider = DynamicModelProvider::from_configs(&file_config.providers)?;

    // Validate all HTTP/SSE servers at startup
    info!("Validating MCP HTTP/SSE servers...");
    let http_servers: Vec<_> = file_config
        .servers
        .iter()
        .filter(|s| s.transport == TransportType::Http)
        .collect();

    if http_servers.is_empty() {
        info!("No HTTP/SSE servers configured");
    } else {
        info!(
            count = http_servers.len(),
            "Found HTTP/SSE servers to validate"
        );

        let mut failed_servers = Vec::new();

        for server_config in &http_servers {
            let url = match &server_config.url {
                Some(url) => url.clone(),
                None => {
                    error!(server = %server_config.name, "HTTP server missing URL");
                    failed_servers.push((server_config.name.clone(), "Missing URL".to_string()));
                    continue;
                }
            };

            info!(server = %server_config.name, url = %url, "Connecting to HTTP/SSE server...");

            let transport_config = HttpTransportConfig {
                name: server_config.name.clone(),
                url,
                headers: server_config.headers.clone(),
            };

            let transport = HttpTransport::new(transport_config);

            match transport.connect().await {
                Ok(()) => {
                    info!(
                        server = %server_config.name,
                        "✓ HTTP/SSE server connected successfully"
                    );
                }
                Err(e) => {
                    error!(
                        server = %server_config.name,
                        error = %e,
                        "✗ Failed to connect to HTTP/SSE server"
                    );
                    failed_servers.push((server_config.name.clone(), e.to_string()));
                }
            }
        }

        if !failed_servers.is_empty() {
            error!("=== SERVER VALIDATION FAILED ===");
            for (name, err) in &failed_servers {
                error!(server = %name, error = %err, "Failed server");
            }
            panic!(
                "Cannot start REST API: {} of {} HTTP/SSE server(s) failed to respond. \
                All servers must be operational before the client can start.",
                failed_servers.len(),
                http_servers.len()
            );
        }

        info!(
            count = http_servers.len(),
            "All HTTP/SSE servers validated successfully"
        );
    }

    // Run server discovery from servers folder
    let _discovery_result =
        antikhitera_mcp_client::application::discovery::run_startup_discovery(None).await;

    let client_config = ClientConfig::new(
        file_config.default_provider.clone(),
        file_config.model.clone(),
    )
    .with_tools(file_config.tools.clone())
    .with_servers(file_config.servers.clone())
    .with_prompts(file_config.prompts.clone())
    .with_providers(file_config.providers.clone());

    let client = Arc::new(McpClient::new(provider, client_config));

    info!(addr = %addr, "REST server starting");
    let cors_origins = &file_config.rest_server.cors_origins;
    let doc_servers = &file_config.rest_server.docs;
    server::serve(client, addr, cors_origins, doc_servers).await?;

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

pub mod application;
pub mod cli;
pub mod config;
pub mod constants;
pub mod domain;
pub mod infrastructure;
pub mod tui;

pub use application::{agent, client, stdio, tooling};
pub use cli::{Cli, RunMode};
pub use config::{AppConfig, ModelProviderConfig};
pub use domain::types;
pub use infrastructure::{model, rpc, server};

use application::client::{ClientConfig, McpClient};
use infrastructure::model::DynamicModelProvider;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt};

pub async fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    let mode = match cli.mode {
        Some(m) => m,
        None => select_mode_interactive()?,
    };
    if mode == RunMode::Setup {
        match tui::screens::run_setup_menu() {
            Ok(_) => {}
            Err(e) => eprintln!("Setup error: {}", e),
        }
        return Box::pin(run(Cli { mode: None, ..cli })).await;
    }
    let config_path = cli.config.as_deref().map(Path::new);
    let default_config = Path::new(config::CONFIG_PATH);
    let check_path = config_path.unwrap_or(default_config);

    if !check_path.exists() {
        println!();
        println!("No configuration found at: {}", check_path.display());
        config::wizard::run_wizard().await?;
    }

    let quiet_mode = matches!(mode, RunMode::Stdio | RunMode::All);
    init_tracing(quiet_mode);
    info!("Starting mcp");
    debug!(
        mode = ?mode,
        config = ?cli.config,
        system = ?cli.system,
        "CLI arguments parsed"
    );

    let file_config = AppConfig::load(config_path)?;
    if let Some(path) = config_path {
        info!(path = %path.display(), "Loaded configuration from file");
    } else {
        info!("Loaded configuration from default path");
    }

    let mut providers = file_config.providers.clone();
    apply_cli_overrides(&cli, &mut providers);
    debug!(
        provider_count = providers.len(),
        "Initializing dynamic model providers"
    );
    let provider = DynamicModelProvider::from_configs(&providers)?;
    let mut client_config = ClientConfig::new(
        file_config.default_provider.clone(),
        file_config.model.clone(),
    )
    .with_tools(file_config.tools.clone())
    .with_servers(file_config.servers.clone())
    .with_prompts(file_config.prompts.clone())
    .with_providers(providers.clone());
    if let Some(system_prompt) = cli.system.clone().or(file_config.system_prompt.clone()) {
        client_config = client_config.with_system_prompt(system_prompt);
    }
    let client = Arc::new(McpClient::new(provider, client_config));

    info!(mode = ?mode, "Running client in selected mode");
    match mode {
        RunMode::Stdio => {
            info!("Launching TUI interactive chat interface");
            let provider_name = file_config.default_provider.clone();
            let model_name = file_config.model.clone();
            match tui::screens::run_chat(client.clone(), provider_name, model_name).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Chat error: {}", e);
                }
            }
        }
        RunMode::Rest => {
            // Use CLI addr if provided, otherwise use config bind address
            let addr: std::net::SocketAddr = cli.rest_addr.unwrap_or_else(|| {
                file_config
                    .rest_server
                    .bind
                    .parse()
                    .expect("Invalid bind address in config")
            });
            info!(addr = %addr, "Starting REST server");
            let cors_origins = &file_config.rest_server.cors_origins;
            let doc_servers = &file_config.rest_server.docs;
            server::serve(client.clone(), addr, cors_origins, doc_servers).await?;
        }
        RunMode::All => {
            // Use CLI addr if provided, otherwise use config bind address
            let rest_addr: std::net::SocketAddr = cli.rest_addr.unwrap_or_else(|| {
                file_config
                    .rest_server
                    .bind
                    .parse()
                    .expect("Invalid bind address in config")
            });
            info!(addr = %rest_addr, "Starting both STDIO and REST server");
            let rest_client = client.clone();
            let cors_origins = file_config.rest_server.cors_origins.clone();
            let doc_servers = file_config.rest_server.docs.clone();
            let rest_handle = tokio::spawn(async move {
                if let Err(e) =
                    server::serve(rest_client, rest_addr, &cors_origins, &doc_servers).await
                {
                    tracing::error!(error = %e, "REST server error");
                }
            });
            let stdio_result = stdio::run(client.clone()).await;
            rest_handle.abort();

            stdio_result?;
        }
        RunMode::Setup => {
            unreachable!("Setup mode should be handled before config loading");
        }
    }
    info!("Client execution finished");
    Ok(())
}

fn select_mode_interactive() -> Result<RunMode, Box<dyn Error>> {
    match tui::screens::run_mode_selector() {
        Ok(Some(mode)) => Ok(mode),
        Ok(None) => {
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("TUI error: {}", e);
            eprintln!("Defaulting to STDIO mode");
            Ok(RunMode::Stdio)
        }
    }
}

fn init_tracing(quiet: bool) {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let filter = if quiet {
            EnvFilter::new("off")
        } else {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        };
        fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_level(true)
            .init();
    });
}

fn apply_cli_overrides(cli: &Cli, providers: &mut [ModelProviderConfig]) {
    for provider in providers.iter_mut() {
        if provider.is_ollama() {
            if provider.endpoint != cli.ollama_url {
                info!(
                    provider = provider.id.as_str(),
                    url = %cli.ollama_url,
                    "Overriding provider endpoint based on CLI flag"
                );
            }
            provider.endpoint = cli.ollama_url.clone();
        }
    }
}

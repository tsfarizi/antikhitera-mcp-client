pub mod application;
pub mod cli;
pub mod config;
pub mod domain;
pub mod infrastructure;

pub use application::{agent, client, stdio, tooling};
pub use cli::{Cli, RunMode};
pub use config::{AppConfig, ModelProviderConfig};
pub use domain::types;
pub use infrastructure::{model, rpc, server};

use application::client::{ClientConfig, McpClient};
use infrastructure::model::DynamicModelProvider;
use std::error::Error;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt};

pub async fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    let mode = match cli.mode {
        Some(m) => m,
        None => select_mode_interactive()?,
    };

    let quiet_mode = matches!(mode, RunMode::Stdio | RunMode::All);
    init_tracing(quiet_mode);
    info!("Starting mcp");
    debug!(
        mode = ?mode,
        config = ?cli.config,
        system = ?cli.system,
        "CLI arguments parsed"
    );

    let config_path = cli.config.as_deref().map(Path::new);
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
    .with_prompt_template(Some(file_config.prompt_template.clone()))
    .with_providers(providers.clone());
    if let Some(system_prompt) = cli.system.clone().or(file_config.system_prompt.clone()) {
        client_config = client_config.with_system_prompt(system_prompt);
    }
    let client = Arc::new(McpClient::new(provider, client_config));

    info!(mode = ?mode, "Running client in selected mode");
    match mode {
        RunMode::Stdio => {
            info!("Launching STDIO interactive chat interface");
            stdio::run(client.clone()).await?;
        }
        RunMode::Rest => {
            info!(addr = %cli.rest_addr, "Starting REST server");
            server::serve(client.clone(), cli.rest_addr).await?;
        }
        RunMode::All => {
            info!(addr = %cli.rest_addr, "Starting both STDIO and REST server");
            let rest_client = client.clone();
            let rest_addr = cli.rest_addr;

            // Spawn REST server in background
            let rest_handle = tokio::spawn(async move {
                if let Err(e) = server::serve(rest_client, rest_addr).await {
                    tracing::error!(error = %e, "REST server error");
                }
            });

            // Run STDIO in foreground
            let stdio_result = stdio::run(client.clone()).await;

            // When STDIO exits, abort REST server
            rest_handle.abort();

            stdio_result?;
        }
    }
    info!("Client execution finished");
    Ok(())
}

fn select_mode_interactive() -> Result<RunMode, Box<dyn Error>> {
    let thread_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    println!();

    if thread_count <= 1 {
        println!("Note: Only 1 thread available, cannot run both modes simultaneously.");
        println!();
        println!("Available modes:");
        println!("  1. STDIO - Interactive chat");
        println!("  2. REST  - API server");
        println!();
        print!("Select mode [1-2]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" | "stdio" => Ok(RunMode::Stdio),
            "2" | "rest" => Ok(RunMode::Rest),
            _ => {
                println!("Invalid selection, defaulting to STDIO");
                Ok(RunMode::Stdio)
            }
        }
    } else {
        println!("Available modes:");
        println!("  1. STDIO - Interactive chat");
        println!("  2. REST  - API server");
        println!("  3. Both  - Run STDIO + REST simultaneously");
        println!();
        print!("Select mode [1-3]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" | "stdio" => Ok(RunMode::Stdio),
            "2" | "rest" => Ok(RunMode::Rest),
            "3" | "both" | "all" => Ok(RunMode::All),
            _ => {
                println!("Invalid selection, defaulting to STDIO");
                Ok(RunMode::Stdio)
            }
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

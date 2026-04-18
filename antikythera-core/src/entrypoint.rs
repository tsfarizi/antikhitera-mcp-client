//! Main entry-point orchestrator.
//!
//! Provides the [`run`] function that ties together config loading, provider
//! initialisation, TUI/REST server dispatch, and the agent runtime.  This is
//! the function the `antikythera` binary (and the CLI crate) calls.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use tracing::{debug, info};

use crate::application::client::{ClientConfig, McpClient};
use crate::cli::{Cli, RunMode};
use crate::config::{AppConfig, ModelProviderConfig};
use crate::infrastructure::model::DynamicModelProvider;

/// Run the MCP client in the mode specified by `cli`.
///
/// Reads configuration from the file system (running the interactive setup
/// wizard if no config exists yet), resolves providers, and dispatches to the
/// TUI chat interface or the REST API server.
pub async fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    let mode = match cli.mode {
        Some(m) => m,
        None => select_mode_interactive()?,
    };

    if mode == RunMode::Setup {
        match crate::tui::screens::run_setup_menu() {
            Ok(_) => {}
            Err(e) => eprintln!("Setup error: {}", e),
        }
        return Box::pin(run(Cli { mode: None, ..cli })).await;
    }

    let config_path = cli.config.as_deref().map(Path::new);
    let default_config = Path::new(crate::config::CONFIG_PATH);
    let check_path = config_path.unwrap_or(default_config);

    if !check_path.exists() {
        println!();
        println!("No configuration found at: {}", check_path.display());
        crate::config::wizard::run_wizard().await?;
    }

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
    debug!(provider_count = providers.len(), "Initializing dynamic model providers");

    let _discovery_result = crate::application::discovery::run_startup_discovery(None).await;

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
            if let Err(e) =
                crate::tui::screens::run_chat(client.clone(), provider_name, model_name).await
            {
                eprintln!("Chat error: {}", e);
            }
        }
        RunMode::Setup => {
            unreachable!("Setup mode is handled before config loading");
        }
    }

    info!("Client execution finished");
    Ok(())
}

fn select_mode_interactive() -> Result<RunMode, Box<dyn Error>> {
    match crate::tui::screens::run_mode_selector() {
        Ok(Some(mode)) => Ok(mode),
        Ok(None) => std::process::exit(0),
        Err(e) => {
            eprintln!("TUI error: {}", e);
            eprintln!("Defaulting to STDIO mode");
            Ok(RunMode::Stdio)
        }
    }
}

fn apply_cli_overrides(cli: &Cli, providers: &mut [ModelProviderConfig]) {
    for provider in providers.iter_mut() {
        if provider.is_ollama() {
            if provider.endpoint != cli.ollama_url {
                info!(
                    provider = provider.id.as_str(),
                    url = %cli.ollama_url,
                    "Overriding Ollama endpoint from CLI flag"
                );
            }
            provider.endpoint = cli.ollama_url.clone();
        }
    }
}

//! TUI module for terminal user interface using Ratatui
//!
//! Provides full-screen interactive menus with arrow key navigation.

pub mod screens;
pub mod terminal;
pub mod theme;
pub mod widgets;

pub use terminal::{NavAction, Tui, restore_terminal};
pub use widgets::{Menu, MenuItem};
pub use widgets::{TableMenu, TableRow, TextInput};

use crate::cli::Cli;
use antikythera_core::application::client::{ClientConfig, McpClient};
use antikythera_core::config::AppConfig;
use antikythera_core::infrastructure::model::DynamicModelProvider;
use std::path::Path;
use std::sync::Arc;

/// Run TUI with CLI arguments
pub async fn run_tui_with_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    use crate::cli::RunMode;
    use tracing::{info, debug};
    use tracing_subscriber::{fmt, EnvFilter};

    let mode = match cli.mode {
        Some(m) => m,
        None => {
            match screens::run_mode_selector()? {
                Some(mode) => mode,
                None => {
                    // User pressed 'q' in mode selector - exit gracefully
                    println!("\n👋 Goodbye!\n");
                    return Ok(());
                }
            }
        }
    };

    // WASM mode doesn't run here - it's a build target
    if mode == RunMode::Wasm {
        println!("WASM mode selected. Use `cargo build --target wasm32-unknown-unknown` to build.");
        return Ok(());
    }

    // CLI mode (default)
    let config_path = cli.config.as_deref().map(Path::new);
    let default_config = Path::new(antikythera_core::constants::CONFIG_PATH);
    let check_path = config_path.unwrap_or(default_config);

    if !check_path.exists() {
        println!();
        println!("No configuration found at: {}", check_path.display());
        crate::wizard::run_wizard().await?;
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).with_target(false).with_level(true).init();

    info!("Starting MCP client in CLI mode");
    debug!(config = ?cli.config, system = ?cli.system, "CLI arguments parsed");

    let file_config = AppConfig::load(config_path)?;
    let provider = DynamicModelProvider::from_configs(&file_config.providers)?;
    
    let client_config = ClientConfig::new(
        file_config.default_provider.clone(),
        file_config.model.clone(),
    )
    .with_tools(file_config.tools.clone())
    .with_servers(file_config.servers.clone())
    .with_prompts(file_config.prompts.clone());

    let client = Arc::new(McpClient::new(provider, client_config));

    info!("Launching TUI chat interface");
    let provider_name = file_config.default_provider.clone();
    let model_name = file_config.model.clone();
    match screens::run_chat(client.clone(), provider_name, model_name).await {
        Ok(_) => {}
        Err(e) => eprintln!("Chat error: {}", e),
    }

    Ok(())
}

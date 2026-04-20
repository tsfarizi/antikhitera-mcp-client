//! CLI Configuration Management Binary
//!
//! Manages the shared `app.pc` configuration file used by both the CLI binary
//! and the core runtime.  Provider, model, and server settings are all stored in
//! a single Postcard blob.

use antikythera_cli::config::*;
use antikythera_cli::error::{CliError, CliResult};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "antikythera-config")]
#[command(about = "Manage Antikythera configuration (app.pc)")]
pub struct ConfigCli {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Initialize default configuration
    Init,
    /// Show all configuration as JSON
    Show,
    /// Get a specific field value
    Get { field: String },
    /// Set a specific field value
    Set { field: String, value: String },
    /// Add a provider
    AddProvider {
        id: String,
        #[arg(name = "type")]
        provider_type: String,
        endpoint: String,
        /// API key environment-variable name (e.g. GEMINI_API_KEY). Omit for Ollama.
        #[arg(name = "api_key")]
        api_key: Option<String>,
    },
    /// Remove a provider by ID
    RemoveProvider { id: String },
    /// Set the default provider and model
    SetModel { provider: String, model: String },
    /// Set the REST server bind address
    SetBind { address: String },
    /// Export configuration as JSON
    Export { output: Option<String> },
    /// Import configuration from JSON
    Import { input: String },
    /// Reset to default configuration
    Reset,
    /// Show config status
    Status,
}

pub fn execute_config_cli(command: ConfigCommand) -> CliResult<()> {
    match command {
        ConfigCommand::Init => {
            if config_exists() {
                println!("Configuration already exists at: {}", CONFIG_PATH);
                println!("Use 'reset' to overwrite.");
                return Ok(());
            }
            init_default_config()?;
            println!("✓ Default configuration created at: {}", CONFIG_PATH);
            Ok(())
        }

        ConfigCommand::Show => {
            let config = load_app_config(None)?;
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);
            Ok(())
        }

        ConfigCommand::Get { field } => {
            let config = load_app_config(None)?;
            let value = get_field(&config, &field)?;
            println!("{}", value);
            Ok(())
        }

        ConfigCommand::Set { field, value } => {
            let mut config = load_app_config(None)?;
            set_field(&mut config, &field, &value)?;
            save_app_config(&config, None)?;
            println!("✓ Set '{}' = '{}'", field, value);
            Ok(())
        }

        ConfigCommand::AddProvider {
            id,
            provider_type,
            endpoint,
            api_key,
        } => {
            let mut config = load_app_config(None)?;

            if config.providers.iter().any(|p| p.id == id) {
                return Err(CliError::Validation(format!(
                    "Provider '{}' already exists",
                    id
                )));
            }

            config.providers.push(ProviderConfig {
                id: id.clone(),
                provider_type,
                endpoint,
                api_key: api_key.unwrap_or_default(),
                models: Vec::new(),
            });

            save_app_config(&config, None)?;
            println!("✓ Provider '{}' added", id);
            Ok(())
        }

        ConfigCommand::RemoveProvider { id } => {
            let mut config = load_app_config(None)?;
            let initial_len = config.providers.len();
            config.providers.retain(|p| p.id != id);

            if config.providers.len() == initial_len {
                Err(CliError::Validation(format!("Provider '{}' not found", id)))
            } else {
                save_app_config(&config, None)?;
                println!("✓ Provider '{}' removed", id);
                Ok(())
            }
        }

        ConfigCommand::SetModel { provider, model } => {
            let mut config = load_app_config(None)?;

            if !config.providers.iter().any(|p| p.id == provider) {
                return Err(CliError::Validation(format!(
                    "Provider '{}' not found",
                    provider
                )));
            }

            config.model.default_provider = provider.clone();
            config.model.model = model.clone();

            save_app_config(&config, None)?;
            println!("✓ Default model set: {} / {}", provider, model);
            Ok(())
        }

        ConfigCommand::SetBind { address } => {
            let mut config = load_app_config(None)?;
            config.server.bind = address.clone();
            save_app_config(&config, None)?;
            println!("✓ Bind address set to: {}", address);
            Ok(())
        }

        ConfigCommand::Export { output } => {
            let config = load_app_config(None)?;
            let json = serde_json::to_string_pretty(&config)?;

            match output {
                Some(path) => {
                    std::fs::write(&path, &json)?;
                    println!("✓ Exported to: {}", path);
                }
                None => println!("{}", json),
            }
            Ok(())
        }

        ConfigCommand::Import { input } => {
            let json = std::fs::read_to_string(&input)?;

            let config: AppConfig = serde_json::from_str(&json)?;

            save_app_config(&config, None)?;
            println!("✓ Imported from: {}", input);
            Ok(())
        }

        ConfigCommand::Reset => {
            init_default_config()?;
            println!("✓ Configuration reset to defaults");
            println!("  Path: {}", CONFIG_PATH);
            Ok(())
        }

        ConfigCommand::Status => {
            if config_exists() {
                let config = load_app_config(None)?;
                println!("✓ Config exists at: {}", CONFIG_PATH);
                println!("  Providers: {}", config.providers.len());
                println!(
                    "  Default: {}/{}",
                    config.model.default_provider, config.model.model
                );
                println!("  Server: {}", config.server.bind);
            } else {
                println!("✗ No config found at: {}", CONFIG_PATH);
                println!("  Run 'init' to create default config.");
            }
            Ok(())
        }
    }
}

fn get_field(config: &AppConfig, field: &str) -> CliResult<String> {
    match field {
        "default_provider" => Ok(config.model.default_provider.clone()),
        "model" => Ok(config.model.model.clone()),
        "server.bind" => Ok(config.server.bind.clone()),
        "providers" => Ok(serde_json::to_string(&config.providers)?),
        _ => Err(CliError::Validation(format!("Unknown field: {}", field))),
    }
}

fn set_field(config: &mut AppConfig, field: &str, value: &str) -> CliResult<()> {
    match field {
        "default_provider" => {
            config.model.default_provider = value.to_string();
            Ok(())
        }
        "model" => {
            config.model.model = value.to_string();
            Ok(())
        }
        "server.bind" => {
            config.server.bind = value.to_string();
            Ok(())
        }
        _ => Err(CliError::Validation(format!("Unknown field: {}", field))),
    }
}

fn main() {
    let args = ConfigCli::parse();
    if let Err(e) = execute_config_cli(args.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

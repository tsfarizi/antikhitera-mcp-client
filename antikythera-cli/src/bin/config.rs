//! CLI Configuration Management Binary
//!
//! Minimal config CLI - only Gemini and Ollama providers.

use antikythera_cli::config::*;
use clap::{Parser, Subcommand};
use serde_json;

#[derive(Parser)]
#[command(name = "antikythera-config")]
#[command(about = "Manage CLI configuration (Postcard format)")]
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
    /// Get a specific field
    Get { field: String },
    /// Set a specific field
    Set { field: String, value: String },
    /// Add a provider (gemini or ollama)
    AddProvider {
        id: String,
        #[arg(name = "type")]
        provider_type: String,
        endpoint: String,
        #[arg(name = "api_key")]
        api_key: Option<String>,
    },
    /// Remove a provider
    RemoveProvider { id: String },
    /// Set default model
    SetModel { provider: String, model: String },
    /// Set server bind address
    SetBind { address: String },
    /// Export configuration as JSON
    Export { output: Option<String> },
    /// Import configuration from JSON
    Import { input: String },
    /// Reset to default configuration
    Reset,
    /// Check config status
    Status,
}

pub fn execute_config_cli(command: ConfigCommand) -> Result<(), String> {
    match command {
        ConfigCommand::Init => {
            if config_exists() {
                println!("Configuration already exists at: {}", CLI_CONFIG_PATH);
                println!("Use 'reset' to overwrite.");
                return Ok(());
            }
            init_default_config()?;
            println!("✓ Default configuration created at: {}", CLI_CONFIG_PATH);
            Ok(())
        }

        ConfigCommand::Show => {
            let config = load_config(None)?;
            let json = serde_json::to_string_pretty(&config)
                .map_err(|e| format!("Failed to serialize: {}", e))?;
            println!("{}", json);
            Ok(())
        }

        ConfigCommand::Get { field } => {
            let config = load_config(None)?;
            let value = get_field(&config, &field)?;
            println!("{}", value);
            Ok(())
        }

        ConfigCommand::Set { field, value } => {
            let mut config = load_config(None)?;
            set_field(&mut config, &field, &value)?;
            save_config(&config, None)?;
            println!("✓ Set '{}' = '{}'", field, value);
            Ok(())
        }

        ConfigCommand::AddProvider { id, provider_type, endpoint, api_key } => {
            // Only Gemini and Ollama supported
            match provider_type.to_lowercase().as_str() {
                "gemini" | "ollama" => {}
                other => return Err(format!("Unsupported provider: {}. Only 'gemini' and 'ollama' are supported.", other)),
            }

            let mut config = load_config(None)?;

            if config.providers.iter().any(|p| p.id == id) {
                return Err(format!("Provider '{}' already exists", id));
            }

            config.providers.push(CliProviderConfig {
                id: id.clone(),
                provider_type,
                endpoint,
                api_key: api_key.unwrap_or_default(),
                models: Vec::new(),
            });

            save_config(&config, None)?;
            println!("✓ Provider '{}' added", id);
            Ok(())
        }

        ConfigCommand::RemoveProvider { id } => {
            let mut config = load_config(None)?;
            let initial_len = config.providers.len();
            config.providers.retain(|p| p.id != id);

            if config.providers.len() == initial_len {
                Err(format!("Provider '{}' not found", id))
            } else {
                save_config(&config, None)?;
                println!("✓ Provider '{}' removed", id);
                Ok(())
            }
        }

        ConfigCommand::SetModel { provider, model } => {
            let mut config = load_config(None)?;

            if !config.providers.iter().any(|p| p.id == provider) {
                return Err(format!("Provider '{}' not found", provider));
            }

            config.default_provider = provider.clone();
            config.model = model.clone();

            save_config(&config, None)?;
            println!("✓ Default model set: {} / {}", provider, model);
            Ok(())
        }

        ConfigCommand::SetBind { address } => {
            let mut config = load_config(None)?;
            config.server.bind = address.clone();
            save_config(&config, None)?;
            println!("✓ Bind address set to: {}", address);
            Ok(())
        }

        ConfigCommand::Export { output } => {
            let config = load_config(None)?;
            let json = serde_json::to_string_pretty(&config)
                .map_err(|e| format!("Failed to serialize: {}", e))?;

            match output {
                Some(path) => {
                    std::fs::write(&path, &json)
                        .map_err(|e| format!("Failed to write: {}", e))?;
                    println!("✓ Exported to: {}", path);
                }
                None => println!("{}", json),
            }
            Ok(())
        }

        ConfigCommand::Import { input } => {
            let json = std::fs::read_to_string(&input)
                .map_err(|e| format!("Failed to read: {}", e))?;

            let config: CliConfig = serde_json::from_str(&json)
                .map_err(|e| format!("Invalid JSON: {}", e))?;

            save_config(&config, None)?;
            println!("✓ Imported from: {}", input);
            Ok(())
        }

        ConfigCommand::Reset => {
            init_default_config()?;
            println!("✓ Configuration reset to defaults");
            println!("  Path: {}", CLI_CONFIG_PATH);
            Ok(())
        }

        ConfigCommand::Status => {
            if config_exists() {
                let config = load_config(None)?;
                println!("✓ Config exists at: {}", CLI_CONFIG_PATH);
                println!("  Providers: {}", config.providers.len());
                println!("  Default: {}/{}", config.default_provider, config.model);
                println!("  Server: {}", config.server.bind);
            } else {
                println!("✗ No config found at: {}", CLI_CONFIG_PATH);
                println!("  Run 'init' to create default config.");
            }
            Ok(())
        }
    }
}

fn get_field(config: &CliConfig, field: &str) -> Result<String, String> {
    match field {
        "default_provider" => Ok(config.default_provider.clone()),
        "model" => Ok(config.model.clone()),
        "server.bind" => Ok(config.server.bind.clone()),
        "providers" => Ok(serde_json::to_string(&config.providers).unwrap()),
        _ => Err(format!("Unknown field: {}", field)),
    }
}

fn set_field(config: &mut CliConfig, field: &str, value: &str) -> Result<(), String> {
    match field {
        "default_provider" => { config.default_provider = value.to_string(); Ok(()) }
        "model" => { config.model = value.to_string(); Ok(()) }
        "server.bind" => { config.server.bind = value.to_string(); Ok(()) }
        _ => Err(format!("Unknown field: {}", field)),
    }
}

fn main() {
    let args = ConfigCli::parse();
    if let Err(e) = execute_config_cli(args.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

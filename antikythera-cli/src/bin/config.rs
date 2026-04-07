//! Configuration CLI Tool
//!
//! Full configuration management via CLI commands.
//! All config is stored as Postcard binary.

use clap::{Parser, Subcommand};
use antikythera_core::config::postcard_config::*;
use antikythera_core::config::migration::*;
use serde_json;

#[derive(Parser)]
#[command(name = "antikythera-config")]
#[command(about = "Manage Antikythera configuration (Postcard format)")]
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
    /// Show configuration size
    Size,
    /// Get a specific config field
    Get {
        /// Field path (e.g., "server.bind", "model.default_provider")
        #[arg(name = "field")]
        field: String,
    },
    /// Set a specific config field
    Set {
        /// Field path (e.g., "server.bind", "model.default_provider")
        #[arg(name = "field")]
        field: String,
        /// New value
        #[arg(name = "value")]
        value: String,
    },
    /// List all providers
    ListProviders,
    /// Add a new provider
    AddProvider {
        /// Provider ID
        #[arg(name = "id")]
        id: String,
        /// Provider type (openai, anthropic, ollama, gemini)
        #[arg(name = "type")]
        provider_type: String,
        /// API endpoint URL
        #[arg(name = "endpoint")]
        endpoint: String,
        /// API key env var name
        #[arg(name = "api_key")]
        api_key: String,
    },
    /// Remove a provider
    RemoveProvider {
        /// Provider ID to remove
        #[arg(name = "id")]
        id: String,
    },
    /// Set default model
    SetModel {
        /// Provider ID
        #[arg(name = "provider")]
        provider: String,
        /// Model name
        #[arg(name = "model")]
        model: String,
    },
    /// List all prompt templates
    ListPrompts,
    /// Get a specific prompt template
    GetPrompt {
        /// Template name
        #[arg(name = "name")]
        name: String,
    },
    /// Set a specific prompt template
    SetPrompt {
        /// Template name
        #[arg(name = "name")]
        name: String,
        /// New template content
        #[arg(name = "value")]
        value: String,
    },
    /// Show agent configuration
    ShowAgent,
    /// Set agent max steps
    SetAgentMaxSteps {
        /// Maximum steps
        #[arg(name = "steps")]
        steps: u32,
    },
    /// Toggle agent verbose logging
    SetAgentVerbose {
        /// Enable verbose logging
        #[arg(name = "enabled")]
        enabled: bool,
    },
    /// Toggle auto-execute tools
    SetAgentAutoExecute {
        /// Enable auto-execute tools
        #[arg(name = "enabled")]
        enabled: bool,
    },
    /// Export configuration as JSON
    Export {
        /// Output file path
        #[arg(name = "output")]
        output: Option<String>,
    },
    /// Import configuration from JSON
    Import {
        /// Input file path
        #[arg(name = "input")]
        input: String,
    },
    /// Reset to default configuration
    Reset,
    /// Migrate from TOML to Postcard
    Migrate,
    /// Check migration status
    MigrationStatus,
    /// Use an existing Postcard config file (copy to project root)
    UseConfig {
        /// Path to existing .pc config file
        #[arg(name = "path")]
        path: String,
    },
    /// Backup current config to a file
    BackupConfig {
        /// Output path for backup (default: config-backup.pc)
        #[arg(name = "output")]
        output: Option<String>,
    },
}

/// Execute CLI command
pub fn execute_config_cli(command: ConfigCommand) -> Result<(), String> {
    match command {
        ConfigCommand::Init => {
            if config_exists() {
                println!("Configuration already exists at: {}", CONFIG_PATH);
                println!("Use 'reset' to overwrite.");
                return Ok(());
            }

            init_default_config(None)?;
            println!("✓ Default configuration created at: {}", CONFIG_PATH);
            println!("  Size: {} bytes", config_size(None)?);
            Ok(())
        }

        ConfigCommand::Show => {
            let config = load_config(None)?;
            let json = serde_json::to_string_pretty(&config)
                .map_err(|e| format!("Failed to serialize config: {}", e))?;
            println!("{}", json);
            Ok(())
        }

        ConfigCommand::Size => {
            let size = config_size(None)?;
            println!("Configuration size: {} bytes", size);
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

        ConfigCommand::ListProviders => {
            let config = load_config(None)?;
            if config.providers.is_empty() {
                println!("No providers configured.");
            } else {
                println!("Providers:");
                for p in &config.providers {
                    println!("  - {} ({})", p.id, p.provider_type);
                    println!("    Endpoint: {}", p.endpoint);
                    if !p.models.is_empty() {
                        println!("    Models:");
                        for m in &p.models {
                            println!("      • {} ({})", m.name, m.display_name);
                        }
                    }
                }
            }
            Ok(())
        }

        ConfigCommand::AddProvider { id, provider_type, endpoint, api_key } => {
            let mut config = load_config(None)?;

            // Check if provider already exists
            if config.providers.iter().any(|p| p.id == id) {
                return Err(format!("Provider '{}' already exists", id));
            }

            config.providers.push(ProviderConfig {
                id,
                provider_type,
                endpoint,
                api_key,
                models: Vec::new(),
            });

            save_config(&config, None)?;
            println!("✓ Provider added");
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

            // Verify provider exists
            if !config.providers.iter().any(|p| p.id == provider) {
                return Err(format!("Provider '{}' not found", provider));
            }

            config.model.default_provider = provider.clone();
            config.model.model = model.clone();

            save_config(&config, None)?;
            println!("✓ Default model set: {} / {}", provider, model);
            Ok(())
        }

        ConfigCommand::ListPrompts => {
            let config = load_config(None)?;
            let prompts = &config.prompts;
            println!("Prompt Templates:");
            println!("  template:");
            println!("    {}", prompts.template.lines().next().unwrap_or(""));
            println!("  tool_guidance: {}", truncate(&prompts.tool_guidance, 60));
            println!("  fallback_guidance: {}", truncate(&prompts.fallback_guidance, 60));
            println!("  json_retry_message: {}", truncate(&prompts.json_retry_message, 60));
            println!("  tool_result_instruction: {}", truncate(&prompts.tool_result_instruction, 60));
            println!("  agent_instructions: {}", truncate(&prompts.agent_instructions, 60));
            println!("  ui_instructions: {}", truncate(&prompts.ui_instructions, 60));
            println!("  language_instructions: {}", truncate(&prompts.language_instructions, 60));
            println!("  agent_max_steps_error: {}", prompts.agent_max_steps_error);
            println!("  no_tools_guidance: {}", prompts.no_tools_guidance);
            Ok(())
        }

        ConfigCommand::GetPrompt { name } => {
            let config = load_config(None)?;
            let value = get_prompt_field(&config.prompts, &name)?;
            println!("{}", value);
            Ok(())
        }

        ConfigCommand::SetPrompt { name, value } => {
            let mut config = load_config(None)?;
            set_prompt_field(&mut config.prompts, &name, &value)?;
            save_config(&config, None)?;
            println!("✓ Prompt '{}' updated", name);
            Ok(())
        }

        ConfigCommand::ShowAgent => {
            let config = load_config(None)?;
            let agent = &config.agent;
            println!("Agent Configuration:");
            println!("  max_steps: {}", agent.max_steps);
            println!("  verbose: {}", agent.verbose);
            println!("  auto_execute_tools: {}", agent.auto_execute_tools);
            println!("  session_timeout_secs: {}", agent.session_timeout_secs);
            Ok(())
        }

        ConfigCommand::SetAgentMaxSteps { steps } => {
            let mut config = load_config(None)?;
            config.agent.max_steps = steps;
            save_config(&config, None)?;
            println!("✓ Agent max steps set to: {}", steps);
            Ok(())
        }

        ConfigCommand::SetAgentVerbose { enabled } => {
            let mut config = load_config(None)?;
            config.agent.verbose = enabled;
            save_config(&config, None)?;
            println!("✓ Agent verbose: {}", enabled);
            Ok(())
        }

        ConfigCommand::SetAgentAutoExecute { enabled } => {
            let mut config = load_config(None)?;
            config.agent.auto_execute_tools = enabled;
            save_config(&config, None)?;
            println!("✓ Agent auto_execute_tools: {}", enabled);
            Ok(())
        }

        ConfigCommand::Export { output } => {
            let config = load_config(None)?;
            let json = serde_json::to_string_pretty(&config)
                .map_err(|e| format!("Failed to serialize: {}", e))?;

            match output {
                Some(path) => {
                    std::fs::write(&path, &json)
                        .map_err(|e| format!("Failed to write file: {}", e))?;
                    println!("✓ Config exported to: {}", path);
                }
                None => {
                    println!("{}", json);
                }
            }
            Ok(())
        }

        ConfigCommand::Import { input } => {
            let json = std::fs::read_to_string(&input)
                .map_err(|e| format!("Failed to read file: {}", e))?;

            let config: AppConfig = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;

            save_config(&config, None)?;
            println!("✓ Config imported from: {}", input);
            Ok(())
        }

        ConfigCommand::Reset => {
            init_default_config(None)?;
            println!("✓ Configuration reset to defaults");
            println!("  Path: {}", CONFIG_PATH);
            println!("  Size: {} bytes", config_size(None)?);
            Ok(())
        }

        ConfigCommand::Migrate => {
            if !needs_migration() {
                println!("No migration needed.");
                if config_exists() {
                    println!("Postcard config already exists at: {}", CONFIG_PATH);
                } else {
                    println!("No TOML config found. Run 'init' to create default config.");
                }
                return Ok(());
            }

            println!("Migrating from TOML to Postcard...");
            let config = migrate_toml_to_postcard()?;
            println!("✓ Migration complete!");
            println!("  New config size: {} bytes", config_size(None)?);
            println!("  Providers: {}", config.providers.len());
            println!("  Default model: {}/{}", config.model.default_provider, config.model.model);
            Ok(())
        }

        ConfigCommand::MigrationStatus => {
            println!("{}", migration_status());
            Ok(())
        }

        ConfigCommand::UseConfig { path } => {
            // Verify source file exists
            if !std::path::Path::new(&path).exists() {
                return Err(format!("Config file not found: {}", path));
            }

            // Copy to project root as app.pc
            let dest = "app.pc";
            std::fs::copy(&path, dest)
                .map_err(|e| format!("Failed to copy config file: {}", e))?;

            println!("✓ Config loaded from: {}", path);
            println!("  Saved as: {}", dest);

            // Verify it's valid
            let size = std::fs::metadata(dest)
                .map_err(|e| format!("Failed to read config: {}", e))?
                .len();
            println!("  Size: {} bytes", size);
            println!("\nVerifying config...");

            match load_config(None) {
                Ok(config) => {
                    println!("  ✓ Config is valid");
                    println!("  Provider: {}/{}", config.model.default_provider, config.model.model);
                    println!("  Providers: {}", config.providers.len());
                    println!("  Agent max steps: {}", config.agent.max_steps);
                }
                Err(e) => {
                    println!("  ✗ Config may be corrupted: {}", e);
                    println!("  Keeping file anyway at: {}", dest);
                }
            }

            Ok(())
        }

        ConfigCommand::BackupConfig { output } => {
            // Load current config
            let config = load_config(None)
                .map_err(|e| format!("Failed to load current config: {}", e))?;

            let backup_path = output.unwrap_or_else(|| "config-backup.pc".to_string());

            // Save to backup file
            save_config(&config, Some(std::path::Path::new(&backup_path)))
                .map_err(|e| format!("Failed to save backup: {}", e))?;

            let size = std::fs::metadata(&backup_path)
                .map_err(|e| format!("Failed to read backup: {}", e))?
                .len();

            println!("✓ Config backed up to: {}", backup_path);
            println!("  Size: {} bytes", size);
            println!("\nThis file can be used later with:");
            println!("  antikythera-config use-config {}", backup_path);

            Ok(())
        }
    }
}

// ============================================================================
// Field Access Helpers
// ============================================================================

fn get_field(config: &AppConfig, field: &str) -> Result<String, String> {
    match field {
        "server.bind" => Ok(config.server.bind.clone()),
        "server.cors_origins" => Ok(serde_json::to_string(&config.server.cors_origins).unwrap()),
        "model.default_provider" => Ok(config.model.default_provider.clone()),
        "model.model" => Ok(config.model.model.clone()),
        "agent.max_steps" => Ok(config.agent.max_steps.to_string()),
        "agent.verbose" => Ok(config.agent.verbose.to_string()),
        "agent.auto_execute_tools" => Ok(config.agent.auto_execute_tools.to_string()),
        "agent.session_timeout_secs" => Ok(config.agent.session_timeout_secs.to_string()),
        "providers" => Ok(serde_json::to_string(&config.providers).unwrap()),
        _ if field.starts_with("prompts.") => {
            let name = field.trim_start_matches("prompts.");
            get_prompt_field(&config.prompts, name)
        }
        _ => Err(format!("Unknown field: {}", field)),
    }
}

fn set_field(config: &mut AppConfig, field: &str, value: &str) -> Result<(), String> {
    match field {
        "server.bind" => {
            config.server.bind = value.to_string();
            Ok(())
        }
        "server.cors_origins" => {
            config.server.cors_origins = serde_json::from_str(value)
                .map_err(|e| format!("Invalid JSON: {}", e))?;
            Ok(())
        }
        "model.default_provider" => {
            config.model.default_provider = value.to_string();
            Ok(())
        }
        "model.model" => {
            config.model.model = value.to_string();
            Ok(())
        }
        "agent.max_steps" => {
            config.agent.max_steps = value.parse()
                .map_err(|e| format!("Invalid number: {}", e))?;
            Ok(())
        }
        "agent.verbose" => {
            config.agent.verbose = value.parse()
                .map_err(|e| format!("Invalid bool: {}", e))?;
            Ok(())
        }
        "agent.auto_execute_tools" => {
            config.agent.auto_execute_tools = value.parse()
                .map_err(|e| format!("Invalid bool: {}", e))?;
            Ok(())
        }
        "agent.session_timeout_secs" => {
            config.agent.session_timeout_secs = value.parse()
                .map_err(|e| format!("Invalid number: {}", e))?;
            Ok(())
        }
        _ if field.starts_with("prompts.") => {
            let name = field.trim_start_matches("prompts.");
            set_prompt_field(&mut config.prompts, name, value)
        }
        _ => Err(format!("Unknown field: {}", field)),
    }
}

fn get_prompt_field(prompts: &PromptsConfig, name: &str) -> Result<String, String> {
    match name {
        "template" => Ok(prompts.template.clone()),
        "tool_guidance" => Ok(prompts.tool_guidance.clone()),
        "fallback_guidance" => Ok(prompts.fallback_guidance.clone()),
        "json_retry_message" => Ok(prompts.json_retry_message.clone()),
        "tool_result_instruction" => Ok(prompts.tool_result_instruction.clone()),
        "agent_instructions" => Ok(prompts.agent_instructions.clone()),
        "ui_instructions" => Ok(prompts.ui_instructions.clone()),
        "language_instructions" => Ok(prompts.language_instructions.clone()),
        "agent_max_steps_error" => Ok(prompts.agent_max_steps_error.clone()),
        "no_tools_guidance" => Ok(prompts.no_tools_guidance.clone()),
        _ => Err(format!("Unknown prompt field: {}", name)),
    }
}

fn set_prompt_field(prompts: &mut PromptsConfig, name: &str, value: &str) -> Result<(), String> {
    match name {
        "template" => prompts.template = value.to_string(),
        "tool_guidance" => prompts.tool_guidance = value.to_string(),
        "fallback_guidance" => prompts.fallback_guidance = value.to_string(),
        "json_retry_message" => prompts.json_retry_message = value.to_string(),
        "tool_result_instruction" => prompts.tool_result_instruction = value.to_string(),
        "agent_instructions" => prompts.agent_instructions = value.to_string(),
        "ui_instructions" => prompts.ui_instructions = value.to_string(),
        "language_instructions" => prompts.language_instructions = value.to_string(),
        "agent_max_steps_error" => prompts.agent_max_steps_error = value.to_string(),
        "no_tools_guidance" => prompts.no_tools_guidance = value.to_string(),
        _ => return Err(format!("Unknown prompt field: {}", name)),
    }
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

fn main() {
    let args = ConfigCli::parse();
    if let Err(e) = execute_config_cli(args.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

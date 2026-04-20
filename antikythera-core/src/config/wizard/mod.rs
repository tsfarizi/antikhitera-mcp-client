//! Configuration wizard module for interactive setup
//!
//! Provides CLI-based configuration when no config file exists.

pub mod generators;
pub mod prompts;
pub mod ui;

use crate::config::postcard_config::{self, ModelConfig, PostcardAppConfig, PromptsConfig};
use generators::client;
use std::error::Error;

/// Run the initial setup wizard when no config exists
pub async fn run_wizard() -> Result<(), Box<dyn Error>> {
    ui::print_header("MCP Client - Configuration Wizard");
    ui::print_info("Welcome! No configuration found.");
    ui::print_info("Let's set up your MCP client.\n");
    ui::print_section("PROVIDER SETUP");

    let provider_type = prompts::prompt_select(
        "Provider Type",
        &["gemini", "ollama", "openai", "anthropic"],
    )?;
    let provider_type_display = to_title_case(&provider_type);
    ui::print_hint(&format!("Saved as: {}", provider_type_display));

    let provider_id = prompts::prompt_text(
        &format!("Provider ID [Enter = {}]", provider_type.to_lowercase()),
        Some(&provider_type.to_lowercase()),
    )?;
    ui::print_hint(&format!("Using: {}", provider_id));

    let default_endpoint = get_default_endpoint(&provider_type);
    let endpoint = prompts::prompt_text("API Endpoint", Some(&default_endpoint))?;

    let api_key_env = format!("{}_API_KEY", provider_type.to_uppercase());
    let api_key = prompts::prompt_password(
        &format!("API Key (saved to .env as {})", api_key_env),
        Some(&api_key_env),
    )?;

    ui::print_section("MODELS");
    let models = prompts::prompt_models()?;

    if models.is_empty() {
        return Err("At least one model is required".into());
    }

    ui::print_section("DEFAULT MODEL");
    let model_names: Vec<&str> = models.iter().map(|(name, _)| name.as_str()).collect();
    ui::print_info(&format!("Available: {}", model_names.join(", ")));
    let default_model = prompts::prompt_text("Default model", Some(&models[0].0))?;

    // Generate .env file
    client::generate_env(&api_key_env, &api_key)?;

    // Build Postcard config
    let config = PostcardAppConfig {
        server: postcard_config::ServerConfig::default(),
        providers: vec![postcard_config::ProviderConfig {
            id: provider_id.clone(),
            provider_type: provider_type_display.clone(),
            endpoint: endpoint.clone(),
            api_key: api_key_env,
            models: models
                .iter()
                .map(|(name, display)| postcard_config::ModelInfo {
                    name: name.clone(),
                    display_name: display.clone(),
                })
                .collect(),
        }],
        model: ModelConfig {
            default_provider: provider_id,
            model: default_model,
        },
        prompts: PromptsConfig::default(),
        agent: postcard_config::AgentConfig::default(),
        custom: std::collections::HashMap::new(),
    };

    // Save as Postcard
    postcard_config::save_config(&config, None)?;

    ui::print_success("Configuration saved!");
    ui::print_info(&format!("  → {}", postcard_config::CONFIG_PATH));
    ui::print_info("  → .env");

    Ok(())
}

/// Run the setup menu (accessible from mode selector)
pub async fn run_setup_menu() -> Result<bool, Box<dyn Error>> {
    loop {
        ui::print_header("Setup Menu");
        println!("  [1] Manage Providers");
        println!("  [2] Manage Prompt Template");
        println!("  [0] Back\n");

        let choice = prompts::prompt_text("Select option", None)?;

        match choice.as_str() {
            "0" => return Ok(true),
            "1" => manage_providers().await?,
            "2" => edit_prompt_template().await?,
            _ => ui::print_error("Invalid option"),
        }
    }
}

async fn manage_providers() -> Result<(), Box<dyn Error>> {
    ui::print_header("Manage Providers");

    let config =
        postcard_config::load_config(None).map_err(|e| format!("Failed to load config: {}", e))?;

    if config.providers.is_empty() {
        ui::print_warning("No providers configured.");
        return Ok(());
    }

    ui::print_info("Current providers:");
    for (i, provider) in config.providers.iter().enumerate() {
        let default_marker = if provider.id == config.model.default_provider {
            " ★"
        } else {
            ""
        };
        println!(
            "  [{}] {} ({}) - {}{}",
            i + 1,
            provider.id,
            provider.provider_type,
            provider.endpoint,
            default_marker
        );
    }
    ui::print_hint(&format!("Default: {}", config.model.default_provider));
    println!();

    println!("  [1] Edit Provider");
    println!("  [2] Set Default Provider");
    println!("  [0] Back\n");

    let choice = prompts::prompt_text("Select action", None)?;

    match choice.as_str() {
        "1" => {
            let select = prompts::prompt_text("Select provider to edit (0 to cancel)", None)?;
            let index: usize = match select.parse::<usize>() {
                Ok(0) => return Ok(()),
                Ok(n) if n <= config.providers.len() => n - 1,
                _ => {
                    ui::print_error("Invalid selection");
                    return Ok(());
                }
            };

            let provider = &config.providers[index];
            ui::print_section(&format!("Editing: {}", provider.id));

            let new_endpoint = prompts::prompt_text("Endpoint", Some(&provider.endpoint))?;
            let current_api_key = &provider.api_key;
            let new_api_key_env = prompts::prompt_text("API Key env var", Some(current_api_key))?;

            client::update_provider(&provider.id, &new_endpoint, &new_api_key_env)?;
            ui::print_success("Provider updated!");
        }
        "2" => {
            let select = prompts::prompt_text("Select provider to set as default", None)?;
            match select.parse::<usize>() {
                Ok(0) => return Ok(()),
                Ok(n) if n <= config.providers.len() => {
                    let provider_id = &config.providers[n - 1].id;
                    let mut cfg = config.clone();
                    cfg.model.default_provider = provider_id.clone();
                    postcard_config::save_config(&cfg, None)?;
                    ui::print_success(&format!("Default provider set to '{}'!", provider_id));
                }
                _ => ui::print_error("Invalid selection"),
            }
        }
        _ => {}
    }

    Ok(())
}

async fn edit_prompt_template() -> Result<(), Box<dyn Error>> {
    ui::print_header("Manage Prompt Template");

    let config =
        postcard_config::load_config(None).map_err(|e| format!("Failed to load config: {}", e))?;

    ui::print_info("Current prompt template:");
    ui::print_divider();
    let template = &config.prompts.template;
    let preview: String = template.lines().take(10).collect::<Vec<&str>>().join("\n");
    println!("{}", preview);
    if template.lines().count() > 10 {
        println!("  ... (truncated)");
    }

    ui::print_divider();
    println!();

    println!("  [1] Replace with default template");
    println!("  [2] Enter new template");
    println!("  [0] Cancel\n");

    let choice = prompts::prompt_text("Select option", None)?;

    match choice.as_str() {
        "1" => {
            let default_template = postcard_config::PromptsConfig::default_template();
            let mut cfg = config.clone();
            cfg.prompts.template = default_template.to_string();
            postcard_config::save_config(&cfg, None)?;
            ui::print_success("Prompt template reset to default!");
        }
        "2" => {
            ui::print_info("Enter new template (empty line to finish):");
            let mut lines = Vec::new();
            loop {
                let line = prompts::prompt_text("", Some(""))?;
                if line.is_empty() {
                    break;
                }
                lines.push(line);
            }
            if !lines.is_empty() {
                let template = lines.join("\n");
                let mut cfg = config.clone();
                cfg.prompts.template = template;
                postcard_config::save_config(&cfg, None)?;
                ui::print_success("Prompt template updated!");
            }
        }
        _ => {}
    }

    Ok(())
}

fn to_title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Generate a human-readable display name from a model identifier
/// Converts kebab-case to Title Case (e.g., "gpt-4-turbo" → "Gpt 4 Turbo")
#[allow(dead_code)]
pub fn generate_display_name(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn get_default_endpoint(provider_type: &str) -> String {
    match provider_type.to_lowercase().as_str() {
        "gemini" => "https://generativelanguage.googleapis.com".to_string(),
        "ollama" => "http://127.0.0.1:11434".to_string(),
        "openai" => "https://api.openai.com".to_string(),
        _ => "https://api.example.com".to_string(),
    }
}

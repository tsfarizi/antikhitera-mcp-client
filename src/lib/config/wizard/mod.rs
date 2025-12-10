//! Configuration wizard module for interactive setup
//!
//! Provides CLI-based configuration when no config file exists,
//! and edit functionality for existing configurations.

pub mod generator;
pub mod generators;
pub mod prompts;
pub mod ui;

use crate::config::{AppConfig, CONFIG_PATH};
use generators::{client, model};
use std::error::Error;
use std::path::Path;

/// Run the initial setup wizard when no config exists
pub async fn run_wizard() -> Result<(), Box<dyn Error>> {
    ui::print_header("MCP Client - Configuration Wizard");
    ui::print_info("Welcome! No configuration found.");
    ui::print_info("Let's set up your MCP client.\n");
    ui::print_section("PROVIDER SETUP");

    let provider_type = prompts::prompt_text("Provider Type (e.g. gemini, ollama, openai)", None)?;
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
    let api_key = prompts::prompt_password(&format!("API Key (saved to .env as {})", api_key_env))?;
    ui::print_section("MODELS");
    let models = prompts::prompt_models()?;

    if models.is_empty() {
        return Err("At least one model is required".into());
    }
    ui::print_section("DEFAULT MODEL");
    let model_names: Vec<&str> = models.iter().map(|(name, _)| name.as_str()).collect();
    ui::print_info(&format!("Available: {}", model_names.join(", ")));
    let default_model = prompts::prompt_text("Default model", Some(&models[0].0))?;

    // Generate client.toml (providers, servers, REST settings)
    client::generate(
        &provider_id,
        &provider_type_display,
        &endpoint,
        &api_key_env,
        &models,
    )?;

    // Generate model.toml (default_provider, model, prompt_template, tools)
    model::generate(&provider_id, &default_model)?;

    // Generate .env file
    client::generate_env(&api_key_env, &api_key)?;

    ui::print_success("Configuration saved!");
    ui::print_info("  → config/client.toml");
    ui::print_info("  → config/model.toml");
    ui::print_info("  → config/.env");

    Ok(())
}

/// Run the setup menu (accessible from mode selector)
pub async fn run_setup_menu() -> Result<bool, Box<dyn Error>> {
    loop {
        ui::print_header("Setup Menu");
        println!("  [1] Manage Providers");
        println!("  [2] Manage Models");
        println!("  [3] Manage MCP Servers");
        println!("  [4] Sync Tools from Servers");
        println!("  [5] Manage Prompt Template");
        println!("  [0] Back\n");

        let choice = prompts::prompt_text("Select option", None)?;

        match choice.as_str() {
            "0" => return Ok(true),
            "1" => manage_providers().await?,
            "2" => manage_models().await?,
            "3" => manage_servers().await?,
            "4" => sync_tools().await?,
            "5" => edit_prompt_template().await?,
            _ => ui::print_error("Invalid option"),
        }
    }
}

async fn manage_providers() -> Result<(), Box<dyn Error>> {
    ui::print_header("Manage Providers");

    let config = load_config()?;

    if config.providers.is_empty() {
        ui::print_warning("No providers configured.");
        return Ok(());
    }
    ui::print_info("Current providers:");
    for (i, provider) in config.providers.iter().enumerate() {
        let default_marker = if provider.id == config.default_provider {
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
    ui::print_hint(&format!("Default: {}", config.default_provider));
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
            let current_api_key = provider.api_key.as_deref().unwrap_or("");
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
                    model::update_default_provider(provider_id)?;
                    ui::print_success(&format!("Default provider set to '{}'!", provider_id));
                }
                _ => ui::print_error("Invalid selection"),
            }
        }
        _ => {}
    }

    Ok(())
}

async fn manage_models() -> Result<(), Box<dyn Error>> {
    ui::print_header("Manage Models");

    let config = load_config()?;

    if config.providers.is_empty() {
        ui::print_warning("No providers configured. Add a provider first.");
        return Ok(());
    }
    ui::print_info("Select provider:");
    for (i, provider) in config.providers.iter().enumerate() {
        let model_count = provider.models.len();
        let default_marker = if provider.id == config.default_provider {
            " ★"
        } else {
            ""
        };
        println!(
            "  [{}] {} ({} models){}",
            i + 1,
            provider.id,
            model_count,
            default_marker
        );
    }
    println!();

    let choice = prompts::prompt_text("Select provider (0 to cancel)", None)?;
    let index: usize = match choice.parse::<usize>() {
        Ok(0) => return Ok(()),
        Ok(n) if n <= config.providers.len() => n - 1,
        _ => {
            ui::print_error("Invalid selection");
            return Ok(());
        }
    };

    let provider = &config.providers[index];
    ui::print_section(&format!("Models for: {}", provider.id));
    if provider.models.is_empty() {
        ui::print_info("No models configured.");
    } else {
        ui::print_info("Current models:");
        for (i, model) in provider.models.iter().enumerate() {
            let display = model.display_name.as_deref().unwrap_or(&model.name);
            let default_marker = if model.name == config.model {
                " ★"
            } else {
                ""
            };
            println!(
                "  [{}] {} ({}){}",
                i + 1,
                model.name,
                display,
                default_marker
            );
        }
        ui::print_hint(&format!("Default model: {}", config.model));
    }

    println!();
    println!("  [1] Add Model");
    println!("  [2] Remove Model");
    println!("  [3] Set Default Model");
    println!("  [0] Back\n");

    let action = prompts::prompt_text("Select action", None)?;

    match action.as_str() {
        "1" => {
            let model_name = prompts::prompt_text("Model name", None)?;
            if model_name.is_empty() {
                return Ok(());
            }
            let display_name = generate_display_name(&model_name);
            ui::print_hint(&format!("Display: {}", display_name));
            client::add_model_to_provider(&provider.id, &model_name, &display_name)?;
            ui::print_success(&format!("Model '{}' added!", model_name));
        }
        "2" => {
            if provider.models.is_empty() {
                ui::print_warning("No models to remove.");
                return Ok(());
            }
            let model_choice = prompts::prompt_text("Select model to remove", None)?;
            let model_idx: usize = match model_choice.parse::<usize>() {
                Ok(0) => return Ok(()),
                Ok(n) if n <= provider.models.len() => n - 1,
                _ => {
                    ui::print_error("Invalid selection");
                    return Ok(());
                }
            };
            let model_name = &provider.models[model_idx].name;
            client::remove_model_from_provider(&provider.id, model_name)?;
            ui::print_success(&format!("Model '{}' removed!", model_name));
        }
        "3" => {
            if provider.models.is_empty() {
                ui::print_warning("No models available.");
                return Ok(());
            }
            let model_choice = prompts::prompt_text("Select model to set as default", None)?;
            match model_choice.parse::<usize>() {
                Ok(0) => return Ok(()),
                Ok(n) if n <= provider.models.len() => {
                    let model_name = &provider.models[n - 1].name;
                    model::update_default_model(model_name)?;
                    ui::print_success(&format!("Default model set to '{}'!", model_name));
                }
                _ => ui::print_error("Invalid selection"),
            }
        }
        _ => {}
    }

    Ok(())
}

async fn manage_servers() -> Result<(), Box<dyn Error>> {
    ui::print_header("MCP Servers");

    let config = load_config()?;

    if config.servers.is_empty() {
        ui::print_info("No servers configured.");
    } else {
        ui::print_info("Current servers:");
        for (i, server) in config.servers.iter().enumerate() {
            println!(
                "  [{}] {} ({})",
                i + 1,
                server.name,
                server.command.display()
            );
        }
    }

    println!();
    println!("  [1] Add Server");
    println!("  [2] Remove Server");
    println!("  [0] Back\n");

    let choice = prompts::prompt_text("Select option", None)?;

    match choice.as_str() {
        "1" => add_server().await?,
        "2" => remove_server(&config).await?,
        _ => {}
    }

    Ok(())
}

async fn add_server() -> Result<(), Box<dyn Error>> {
    ui::print_section("Add MCP Server");

    let name = prompts::prompt_text("Server name", None)?;
    if name.is_empty() {
        return Ok(());
    }

    let command = prompts::prompt_text("Command (path to executable)", None)?;
    if command.is_empty() {
        ui::print_error("Command is required");
        return Ok(());
    }

    let args = prompts::prompt_text("Arguments (comma-separated, or empty)", Some(""))?;

    let args_vec: Vec<String> = if args.is_empty() {
        vec![]
    } else {
        args.split(',').map(|s| s.trim().to_string()).collect()
    };

    client::add_server(&name, &command, &args_vec)?;
    ui::print_success(&format!("Server '{}' added!", name));
    if prompts::prompt_confirm("Sync tools from this server now?", true)? {
        sync_tools_from_single_server(&name, &command, &args_vec).await?;
    }

    Ok(())
}

async fn remove_server(config: &AppConfig) -> Result<(), Box<dyn Error>> {
    if config.servers.is_empty() {
        ui::print_warning("No servers to remove.");
        return Ok(());
    }

    ui::print_section("Remove Server");
    ui::print_info("Select server to remove:");
    for (i, server) in config.servers.iter().enumerate() {
        println!("  [{}] {}", i + 1, server.name);
    }
    println!();

    let choice = prompts::prompt_text("Select server (0 to cancel)", None)?;
    let index: usize = match choice.parse::<usize>() {
        Ok(0) => return Ok(()),
        Ok(n) if n <= config.servers.len() => n - 1,
        _ => {
            ui::print_error("Invalid selection");
            return Ok(());
        }
    };

    let server_name = &config.servers[index].name;

    if prompts::prompt_confirm(&format!("Remove server '{}'?", server_name), false)? {
        client::remove_server(server_name)?;
        ui::print_success(&format!("Server '{}' removed!", server_name));
    }

    Ok(())
}

async fn sync_tools() -> Result<(), Box<dyn Error>> {
    ui::print_header("Sync Tools from Servers");

    let config = load_config()?;

    if config.servers.is_empty() {
        ui::print_warning("No servers configured. Add a server first.");
        return Ok(());
    }

    ui::print_info("Available servers:");
    for (i, server) in config.servers.iter().enumerate() {
        println!("  [{}] {}", i + 1, server.name);
    }
    println!("  [A] Sync all servers");
    println!();

    let choice = prompts::prompt_text("Select server (0 to cancel)", None)?;

    if choice.to_lowercase() == "a" {
        ui::print_info("\nSyncing tools from all servers...");
        for server in &config.servers {
            let args: Vec<String> = server.args.clone();
            sync_tools_from_single_server(
                &server.name,
                server.command.to_str().unwrap_or(""),
                &args,
            )
            .await?;
        }
    } else {
        match choice.parse::<usize>() {
            Ok(0) => return Ok(()),
            Ok(n) if n <= config.servers.len() => {
                let server = &config.servers[n - 1];
                let args: Vec<String> = server.args.clone();
                sync_tools_from_single_server(
                    &server.name,
                    server.command.to_str().unwrap_or(""),
                    &args,
                )
                .await?;
            }
            _ => {
                ui::print_error("Invalid selection");
            }
        }
    }

    Ok(())
}

async fn sync_tools_from_single_server(
    name: &str,
    command: &str,
    args: &[String],
) -> Result<(), Box<dyn Error>> {
    ui::print_info(&format!("\nConnecting to server '{}'...", name));
    use crate::config::ServerConfig;
    use crate::tooling::spawn_and_list_tools;
    use std::collections::HashMap;
    use std::path::PathBuf;

    let server_config = ServerConfig {
        name: name.to_string(),
        command: PathBuf::from(command),
        args: args.to_vec(),
        env: HashMap::new(),
        workdir: None,
        default_timezone: None,
        default_city: None,
    };
    match spawn_and_list_tools(&server_config).await {
        Ok(tools) => {
            if tools.is_empty() {
                ui::print_warning(&format!("Server '{}' has no tools.", name));
            } else {
                ui::print_info(&format!("Found {} tools from '{}':", tools.len(), name));
                let mut tool_data = Vec::new();
                for (tool_name, description) in &tools {
                    let desc_preview: String = description.chars().take(50).collect();
                    println!("    • {} - {}", tool_name, desc_preview);
                    tool_data.push((tool_name.clone(), description.clone()));
                }
                model::sync_tools_from_server(name, tool_data)?;
                ui::print_success(&format!("Synced {} tools from '{}'!", tools.len(), name));
            }
        }
        Err(e) => {
            ui::print_error(&format!("Failed to connect to server: {}", e));
        }
    }

    Ok(())
}

async fn edit_prompt_template() -> Result<(), Box<dyn Error>> {
    ui::print_header("Manage Prompt Template");

    let config = load_config()?;

    ui::print_info("Current prompt template:");
    ui::print_divider();
    let template = config.prompt_template();
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
            let default_template = r#"You are a helpful AI assistant.

{{custom_instruction}}

{{language_guidance}}

{{tool_guidance}}"#;
            model::update_prompt_template(default_template)?;
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
                model::update_prompt_template(&template)?;
                ui::print_success("Prompt template updated!");
            }
        }
        _ => {}
    }

    Ok(())
}

fn load_config() -> Result<AppConfig, Box<dyn Error>> {
    AppConfig::load(Some(Path::new(CONFIG_PATH))).map_err(|e| Box::new(e) as Box<dyn Error>)
}

fn to_title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

fn generate_display_name(name: &str) -> String {
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

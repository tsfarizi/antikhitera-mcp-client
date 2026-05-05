//! Configuration wizard module for interactive setup
//!
//! Provides CLI-based configuration when no config file exists.

pub mod generators;
pub mod prompts;
pub mod ui;

use crate::config::postcard_config;
use generators::client;
use std::error::Error;

/// Run the setup menu (accessible from mode selector)
pub async fn run_setup_menu() -> Result<bool, Box<dyn Error>> {
    loop {
        ui::print_header("Setup Menu");
        antikythera_log::cli_print!("  [1] Manage Providers");
        antikythera_log::cli_print!("  [2] Manage Prompt Template");
        antikythera_log::cli_print!("  [0] Back\n");

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
        antikythera_log::cli_print!(
            "  [{}] {} ({}) - {}{}",
            i + 1,
            provider.id,
            provider.provider_type,
            provider.endpoint,
            default_marker
        );
    }
    ui::print_hint(&format!("Default: {}", config.model.default_provider));
    antikythera_log::cli_print!();

    antikythera_log::cli_print!("  [1] Edit Provider");
    antikythera_log::cli_print!("  [2] Set Default Provider");
    antikythera_log::cli_print!("  [0] Back\n");

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
    antikythera_log::cli_print!("{}", preview);
    if template.lines().count() > 10 {
        antikythera_log::cli_print!("  ... (truncated)");
    }

    ui::print_divider();
    antikythera_log::cli_print!();

    antikythera_log::cli_print!("  [1] Replace with default template");
    antikythera_log::cli_print!("  [2] Enter new template");
    antikythera_log::cli_print!("  [0] Cancel\n");

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

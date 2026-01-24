//! Sync tools from MCP servers
//!
//! Provides TUI interface for syncing tools from MCP servers.

mod display;

use super::load_config;
use crate::config::ServerConfig;
use crate::config::wizard::generators::model;
use crate::tooling::spawn_and_list_tools;
use crate::tui::terminal::Tui;
use ratatui::style::Color;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;

use display::{show_all_results_tui, show_result_tui, show_status_tui};

/// Sync tools from a single server (TUI version)
pub fn run_sync_single_server_tui(
    terminal: &mut Tui,
    server: &ServerConfig,
) -> Result<(), Box<dyn Error>> {
    show_status_tui(
        terminal,
        &format!("🔄 Syncing: {}", server.name),
        "⏳ Connecting to server...",
    )?;
    let config = load_config()?;
    let existing_tools: HashSet<String> = config
        .tools
        .iter()
        .filter(|t| t.server.as_deref() == Some(&server.name))
        .map(|t| t.name.clone())
        .collect();

    let handle = tokio::runtime::Handle::current();
    let result = tokio::task::block_in_place(|| handle.block_on(spawn_and_list_tools(server)));

    match result {
        Ok(tools) => {
            if tools.is_empty() {
                show_result_tui(
                    terminal,
                    &server.name,
                    "⚠️ No tools found",
                    0,
                    0,
                    Color::Yellow,
                )?;
            } else {
                let mut tool_data = Vec::new();
                let mut new_count = 0;

                for (tool_name, description) in &tools {
                    if !existing_tools.contains(tool_name) {
                        new_count += 1;
                    }
                    tool_data.push((tool_name.clone(), description.clone()));
                }
                model::sync_tools_from_server(&server.name, tool_data)?;

                show_result_tui(
                    terminal,
                    &server.name,
                    "✓ Sync complete!",
                    tools.len(),
                    new_count,
                    Color::Green,
                )?;
            }
        }
        Err(e) => {
            show_result_tui(
                terminal,
                &server.name,
                &format!("❌ Failed: {}", e),
                0,
                0,
                Color::Red,
            )?;
        }
    }

    Ok(())
}

/// Sync tools from all servers (TUI version)
pub fn run_sync_all_servers_tui(
    terminal: &mut Tui,
    config: &crate::config::AppConfig,
) -> Result<(), Box<dyn Error>> {
    let mut results: Vec<(String, bool, usize, usize)> = Vec::new();

    for server in &config.servers {
        show_status_tui(
            terminal,
            "🔄 Syncing All Servers",
            &format!("⏳ Syncing: {}...", server.name),
        )?;
        let current_config = load_config()?;
        let existing_tools: HashSet<String> = current_config
            .tools
            .iter()
            .filter(|t| t.server.as_deref() == Some(&server.name))
            .map(|t| t.name.clone())
            .collect();

        let server_config = ServerConfig {
            name: server.name.clone(),
            transport: server.transport.clone(),
            command: server.command.clone(),
            args: server.args.clone(),
            env: HashMap::new(),
            workdir: None,
            url: server.url.clone(),
            headers: server.headers.clone(),
            default_timezone: None,
            default_city: None,
        };

        let handle = tokio::runtime::Handle::current();
        let result =
            tokio::task::block_in_place(|| handle.block_on(spawn_and_list_tools(&server_config)));

        match result {
            Ok(tools) => {
                if tools.is_empty() {
                    results.push((server.name.clone(), true, 0, 0));
                } else {
                    let new_count: usize = tools
                        .iter()
                        .filter(|(name, _)| !existing_tools.contains(name))
                        .count();

                    let tool_data: Vec<(String, String)> = tools.into_iter().collect();
                    let total = tool_data.len();
                    model::sync_tools_from_server(&server.name, tool_data)?;

                    results.push((server.name.clone(), true, total, new_count));
                }
            }
            Err(_) => {
                results.push((server.name.clone(), false, 0, 0));
            }
        }
    }
    show_all_results_tui(terminal, &results)?;

    Ok(())
}

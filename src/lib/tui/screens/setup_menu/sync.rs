//! Sync tools from MCP servers

use super::load_config;
use crate::config::ServerConfig;
use crate::config::wizard::generator;
use crate::tooling::spawn_and_list_tools;
use crate::tui::terminal::{NavAction, Tui, read_key};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;

/// Sync tools from a single server (TUI version)
pub fn run_sync_single_server_tui(
    terminal: &mut Tui,
    name: &str,
    command: &str,
    args: &[String],
) -> Result<(), Box<dyn Error>> {
    show_status_tui(
        terminal,
        &format!("🔄 Syncing: {}", name),
        "⏳ Connecting to server...",
    )?;
    let config = load_config()?;
    let existing_tools: HashSet<String> = config
        .tools
        .iter()
        .filter(|t| t.server.as_deref() == Some(name))
        .map(|t| t.name.clone())
        .collect();

    let server_config = ServerConfig {
        name: name.to_string(),
        command: PathBuf::from(command),
        args: args.to_vec(),
        env: HashMap::new(),
        workdir: None,
        default_timezone: None,
        default_city: None,
    };
    let handle = tokio::runtime::Handle::current();
    let result =
        tokio::task::block_in_place(|| handle.block_on(spawn_and_list_tools(&server_config)));

    match result {
        Ok(tools) => {
            if tools.is_empty() {
                show_result_tui(terminal, name, "⚠️ No tools found", 0, 0, Color::Yellow)?;
            } else {
                let mut tool_data = Vec::new();
                let mut new_count = 0;

                for (tool_name, description) in &tools {
                    if !existing_tools.contains(tool_name) {
                        new_count += 1;
                    }
                    tool_data.push((tool_name.clone(), description.clone()));
                }
                generator::sync_tools_from_server(name, tool_data)?;

                show_result_tui(
                    terminal,
                    name,
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
                name,
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
            command: server.command.clone(),
            args: server.args.clone(),
            env: HashMap::new(),
            workdir: None,
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
                    generator::sync_tools_from_server(&server.name, tool_data)?;

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

/// Show status during sync
fn show_status_tui(terminal: &mut Tui, title: &str, status: &str) -> Result<(), Box<dyn Error>> {
    terminal.draw(|frame| {
        let area = frame.area();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" {} ", title));

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(status, Style::default().fg(Color::Yellow))),
            Line::from(""),
        ];

        let para = Paragraph::new(text).block(block);
        frame.render_widget(para, area);
    })?;
    Ok(())
}

/// Show single server sync result
fn show_result_tui(
    terminal: &mut Tui,
    server: &str,
    status: &str,
    total: usize,
    new_count: usize,
    color: Color,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color))
                .title(format!(" 🔄 Sync: {} ", server));

            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    status,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];

            if total > 0 {
                lines.push(Line::from(format!("📊 Total tools: {}", total)));
                lines.push(Line::from(format!("🆕 New tools: {}", new_count)));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press Enter to continue...",
                Style::default().fg(Color::DarkGray),
            )));

            let para = Paragraph::new(lines).block(block);
            frame.render_widget(para, area);
        })?;

        let action = NavAction::from(read_key()?);
        if matches!(
            action,
            NavAction::Select | NavAction::Back | NavAction::ForceQuit
        ) {
            break;
        }
    }
    Ok(())
}

/// Show all servers sync results
fn show_all_results_tui(
    terminal: &mut Tui,
    results: &[(String, bool, usize, usize)],
) -> Result<(), Box<dyn Error>> {
    let success_count = results.iter().filter(|(_, ok, _, _)| *ok).count();
    let fail_count = results.iter().filter(|(_, ok, _, _)| !*ok).count();
    let total_tools: usize = results.iter().map(|(_, _, t, _)| t).sum();
    let total_new: usize = results.iter().map(|(_, _, _, n)| n).sum();

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(" 📊 Sync Summary ");

            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "✓ All servers synced!",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];

            for (name, ok, total, new_count) in results {
                let icon = if *ok { "✓" } else { "❌" };
                let color = if *ok { Color::Green } else { Color::Red };
                lines.push(Line::from(Span::styled(
                    format!("{} {} - {} tools ({} new)", icon, name, total, new_count),
                    Style::default().fg(color),
                )));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")));
            lines.push(Line::from(format!(
                "Servers synced: {} | Failed: {}",
                success_count, fail_count
            )));
            lines.push(Line::from(format!(
                "Total tools: {} | New: {}",
                total_tools, total_new
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press Enter to continue...",
                Style::default().fg(Color::DarkGray),
            )));

            let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
            frame.render_widget(para, area);
        })?;

        let action = NavAction::from(read_key()?);
        if matches!(
            action,
            NavAction::Select | NavAction::Back | NavAction::ForceQuit
        ) {
            break;
        }
    }
    Ok(())
}

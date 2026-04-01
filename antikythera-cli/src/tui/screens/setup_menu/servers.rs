//! Server management TUI

use super::load_config;
use super::sync::{run_sync_all_servers_tui, run_sync_single_server_tui};
use crate::tui::TableRow;
use crate::tui::terminal::{NavAction, Tui, read_key};
use crate::tui::widgets::{Menu, MenuItem, TableMenu};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::error::Error;

/// Manage MCP servers screen (uses existing terminal)
pub fn run_manage_servers_with_terminal(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    let mut selected_idx: usize = 0;

    loop {
        let config = load_config()?;
        let rows: Vec<TableRow> = config
            .servers
            .iter()
            .map(|s| {
                let cmd_display = s
                    .command
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .or_else(|| s.url.as_deref())
                    .unwrap_or("");
                TableRow::new(vec![s.name.clone(), truncate_path(cmd_display, 30)])
            })
            .collect();

        let mut menu = TableMenu::new(
            "🖥️  Manage MCP Servers",
            vec!["Name".into(), "Command".into()],
            rows,
            vec!["🔄 Sync All Servers".into(), "← Back".into()],
        )
        .with_subtitle("Select server to view details & sync tools");
        menu.selected = selected_idx.min(menu.total_items().saturating_sub(1));

        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => {
                menu.previous();
                selected_idx = menu.selected_index();
            }
            NavAction::Down => {
                menu.next();
                selected_idx = menu.selected_index();
            }
            NavAction::Select => {
                if menu.is_row_selected() {
                    let server = config.servers[menu.selected_index()].clone();
                    run_server_details_tui(terminal, &server, &config)?;
                } else if let Some(action_idx) = menu.selected_action_index() {
                    match action_idx {
                        0 => {
                            run_sync_all_servers_tui(terminal, &config)?;
                        }
                        1 => break, // Back
                        _ => {}
                    }
                }
            }
            NavAction::ForceQuit | NavAction::Back => break,
            NavAction::None => {}
        }
    }

    Ok(())
}

/// Truncate a file path to show only the last N characters with ... prefix
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        let chars: Vec<char> = path.chars().collect();
        let start = path.len().saturating_sub(max_len - 3);
        format!("...{}", &chars[start..].iter().collect::<String>())
    }
}

/// Show detailed server info in TUI
fn run_server_details_tui(
    terminal: &mut Tui,
    server: &antikythera_core::config::ServerConfig,
    config: &antikythera_core::config::AppConfig,
) -> Result<(), Box<dyn Error>> {
    let server_tools: Vec<_> = config
        .tools
        .iter()
        .filter(|t| t.server.as_deref() == Some(&server.name))
        .collect();
    let mut lines = vec![
        Line::from(vec![
            Span::styled("📛 Name:    ", Style::default().fg(Color::Yellow)),
            Span::raw(&server.name),
        ]),
        Line::from(vec![
            Span::styled("📂 Command: ", Style::default().fg(Color::Yellow)),
            Span::raw(
                server
                    .command
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .or_else(|| server.url.clone())
                    .unwrap_or_else(|| "(none)".to_string()),
            ),
        ]),
        Line::from(vec![
            Span::styled("📋 Args:    ", Style::default().fg(Color::Yellow)),
            Span::raw(if server.args.is_empty() {
                "(none)".to_string()
            } else {
                server.args.join(" ")
            }),
        ]),
    ];

    if let Some(workdir) = &server.workdir {
        lines.push(Line::from(vec![
            Span::styled("📁 Workdir: ", Style::default().fg(Color::Yellow)),
            Span::raw(workdir.to_string_lossy().to_string()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("🔧 Tools synced ({})", server_tools.len()),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    if server_tools.is_empty() {
        lines.push(Line::from(Span::styled(
            "   (No tools synced yet)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for tool in server_tools.iter().take(8) {
            let desc = tool.description.as_deref().unwrap_or("-");
            let desc_preview: String = desc.chars().take(30).collect();
            lines.push(Line::from(format!("   • {} - {}", tool.name, desc_preview)));
        }
        if server_tools.len() > 8 {
            lines.push(Line::from(Span::styled(
                format!("   ... and {} more", server_tools.len() - 8),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
    let action_items = vec![
        MenuItem::new("🔄 Sync tools from this server"),
        MenuItem::new("← Back"),
    ];

    let mut action_menu = Menu::new(format!("🖥️ Server: {}", server.name), action_items);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::vertical([Constraint::Min(15), Constraint::Length(8)]).split(area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Server Details ");

            let para = Paragraph::new(lines.clone())
                .block(block)
                .wrap(Wrap { trim: true });
            frame.render_widget(para, chunks[0]);
            action_menu.render(frame, chunks[1]);
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => action_menu.previous(),
            NavAction::Down => action_menu.next(),
            NavAction::Select => {
                match action_menu.selected_index() {
                    Some(0) => {
                        run_sync_single_server_tui(terminal, server)?;
                        break; // Return to refresh
                    }
                    Some(1) => break, // Back
                    _ => {}
                }
            }
            NavAction::ForceQuit | NavAction::Back => break,
            NavAction::None => {}
        }
    }

    Ok(())
}

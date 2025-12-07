//! Provider management TUI

use super::load_config;
use crate::tui::TableRow;
use crate::tui::terminal::{NavAction, Tui, read_key};
use crate::tui::widgets::{Menu, MenuItem, TableMenu};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::error::Error;

/// Manage providers screen (uses existing terminal)
pub fn run_manage_providers_with_terminal(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    use crate::config::wizard::generator;

    let mut selected_idx: usize = 0;

    loop {
        let config = load_config()?;

        // Build table rows - ID | Type | Endpoint
        let rows: Vec<TableRow> = config
            .providers
            .iter()
            .map(|p| {
                TableRow::new(vec![
                    p.id.clone(),
                    p.provider_type.clone(),
                    p.endpoint.clone(),
                ])
                .with_default_marker(p.id == config.default_provider)
            })
            .collect();

        let mut menu = TableMenu::new(
            "📦 Manage Providers",
            vec!["ID".into(), "Type".into(), "Endpoint".into()],
            rows,
            vec!["+ Add Provider(s)".into(), "← Back".into()],
        )
        .with_subtitle(format!(
            "Default: {} │ Enter on provider = set default │ Enter on action = execute",
            config.default_provider
        ));
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
                    // Set this provider as default
                    let provider_id = &config.providers[menu.selected_index()].id;
                    generator::update_default_provider(provider_id)?;
                } else if let Some(action_idx) = menu.selected_action_index() {
                    match action_idx {
                        0 => {
                            // Add Provider(s) - stay in TUI
                            run_add_providers_tui(terminal)?;
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

/// Add providers using TUI (no native terminal)
fn run_add_providers_tui(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    use crate::config::wizard::generator;

    // Show provider type selection menu
    let provider_types = vec![
        (
            "Gemini (Google)",
            "gemini",
            "https://generativelanguage.googleapis.com",
        ),
        ("OpenAI", "openai", "https://api.openai.com"),
        ("Ollama (Local)", "ollama", "http://localhost:11434"),
        ("Anthropic", "anthropic", "https://api.anthropic.com"),
        ("Custom", "custom", ""),
    ];

    let items: Vec<MenuItem> = provider_types
        .iter()
        .map(|(name, _, _)| MenuItem::new(*name))
        .chain(std::iter::once(MenuItem::new("← Cancel")))
        .collect();

    let mut menu = Menu::new("+ Add Provider", items).with_subtitle("Select provider type to add");

    loop {
        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => menu.previous(),
            NavAction::Down => menu.next(),
            NavAction::Select => {
                if let Some(idx) = menu.selected_index() {
                    if idx < provider_types.len() {
                        let (_, ptype, endpoint) = provider_types[idx];

                        // Generate a unique ID for the provider
                        let config = load_config()?;
                        let id = format!("{}-{}", ptype, config.providers.len() + 1);

                        // Add the provider
                        generator::add_provider(
                            &id,
                            ptype,
                            endpoint,
                            Some(&format!("{}_API_KEY", ptype.to_uppercase())),
                        )?;

                        // Show success message
                        show_message_tui(
                            terminal,
                            &format!("✓ Provider '{}' added!", id),
                            Color::Green,
                        )?;
                        break;
                    } else {
                        break; // Cancel
                    }
                }
            }
            NavAction::ForceQuit | NavAction::Back => break,
            NavAction::None => {}
        }
    }

    Ok(())
}

/// Show a message in TUI
fn show_message_tui(terminal: &mut Tui, message: &str, color: Color) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|frame| {
            let area = centered_rect(50, 20, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color))
                .title(" Message ");

            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    message,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press Enter to continue...",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let para = Paragraph::new(text).block(block);
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

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

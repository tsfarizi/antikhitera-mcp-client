//! Model management TUI

use super::load_config;
use crate::tui::TableRow;
use crate::tui::terminal::{NavAction, Tui, read_key};
use crate::tui::widgets::{Menu, MenuItem, TableMenu};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::error::Error;

/// Manage models screen (uses existing terminal)
pub fn run_manage_models_with_terminal(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    use crate::config::wizard::generators::model;
    let provider_idx: usize;
    let mut prov_selected: usize = 0;
    loop {
        let config = load_config()?;

        let items: Vec<MenuItem> = config
            .providers
            .iter()
            .map(|p| {
                MenuItem::new(format!("{} ({} models)", p.id, p.models.len()))
                    .with_default_marker(p.id == config.default_provider)
            })
            .chain(std::iter::once(MenuItem::new("← Back")))
            .collect();

        let mut menu = Menu::new("🎯 Manage Models - Select Provider", items)
            .with_subtitle("Select provider to manage models");
        menu.select(prov_selected.min(config.providers.len()));

        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => {
                menu.previous();
                prov_selected = menu.selected_index().unwrap_or(0);
            }
            NavAction::Down => {
                menu.next();
                prov_selected = menu.selected_index().unwrap_or(0);
            }
            NavAction::Select => {
                if let Some(idx) = menu.selected_index() {
                    if idx == config.providers.len() {
                        return Ok(()); // Back
                    }
                    provider_idx = idx;
                    break;
                }
            }
            NavAction::ForceQuit | NavAction::Back => {
                return Ok(());
            }
            NavAction::None => {}
        }
    }
    let mut model_selected: usize = 0;
    loop {
        let config = load_config()?;
        let provider = &config.providers[provider_idx];
        let rows: Vec<TableRow> = provider
            .models
            .iter()
            .map(|m| {
                TableRow::new(vec![
                    m.name.clone(),
                    m.display_name.clone().unwrap_or_else(|| m.name.clone()),
                ])
                .with_default_marker(m.name == config.model)
            })
            .collect();

        let mut menu = TableMenu::new(
            format!("🎯 Models for: {}", provider.id),
            vec!["Model Name".into(), "Display Name".into()],
            rows,
            vec!["+ Add Model".into(), "← Back".into()],
        )
        .with_subtitle(format!(
            "Default: {} │ Enter on model = set default",
            config.model
        ));
        menu.selected = model_selected.min(menu.total_items().saturating_sub(1));

        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => {
                menu.previous();
                model_selected = menu.selected_index();
            }
            NavAction::Down => {
                menu.next();
                model_selected = menu.selected_index();
            }
            NavAction::Select => {
                if menu.is_row_selected() {
                    let model_name = &provider.models[menu.selected_index()].name;
                    model::update_default_model(model_name)?;
                } else if let Some(action_idx) = menu.selected_action_index() {
                    match action_idx {
                        0 => {
                            let provider_id = provider.id.clone();
                            run_add_models_tui(terminal, &provider_id)?;
                        }
                        1 => break, // Back to provider selection
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

/// Add models using TUI (no native terminal)
fn run_add_models_tui(terminal: &mut Tui, provider_id: &str) -> Result<(), Box<dyn Error>> {
    use crate::config::wizard::generators::client;
    let model_presets = vec![
        "gemini-2.0-flash-exp",
        "gemini-1.5-pro",
        "gemini-1.5-flash",
        "gpt-4o",
        "gpt-4o-mini",
        "claude-3-5-sonnet",
        "llama3.2",
        "qwen2.5-coder",
    ];

    let items: Vec<MenuItem> = model_presets
        .iter()
        .map(|name| MenuItem::new(*name))
        .chain(std::iter::once(MenuItem::new("← Cancel")))
        .collect();

    let mut menu = Menu::new(format!("+ Add Model to {}", provider_id), items)
        .with_subtitle("Select a model preset to add");

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
                    if idx < model_presets.len() {
                        let model_name = model_presets[idx];
                        client::add_model_to_provider(provider_id, model_name, model_name)?;
                        show_message_tui(
                            terminal,
                            &format!("✓ Model '{}' added!", model_name),
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

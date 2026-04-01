//! Dialog widgets for prompts
//!
//! Provides confirmation dialogs and message boxes.

use crate::tui::terminal::{NavAction, Tui, read_key};
use crate::tui::widgets::{Menu, MenuItem};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::error::Error;

/// Reset field confirmation
pub fn run_reset_field_confirmation(
    terminal: &mut Tui,
    field_label: &str,
) -> Result<bool, Box<dyn Error>> {
    let items = vec![
        MenuItem::new("❌ Cancel"),
        MenuItem::new(format!("🔄 Yes, Reset {}", field_label)),
    ];
    let mut menu = Menu::new(&format!("Reset {}?", field_label), items)
        .with_subtitle("This will replace the current value with the default.");

    loop {
        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => menu.previous(),
            NavAction::Down => menu.next(),
            NavAction::Select => match menu.selected_index() {
                Some(0) => return Ok(false),
                Some(1) => return Ok(true),
                _ => {}
            },
            NavAction::ForceQuit | NavAction::Back => return Ok(false),
            NavAction::None => {}
        }
    }
}

/// Reset all confirmation
pub fn run_reset_all_confirmation(terminal: &mut Tui) -> Result<bool, Box<dyn Error>> {
    let items = vec![
        MenuItem::new("❌ Cancel"),
        MenuItem::new("⚠️  Yes, Reset ALL Prompts"),
    ];
    let mut menu = Menu::new("⚠️  Reset All Prompts?", items)
        .with_subtitle("This will reset ALL prompt fields to their defaults.");

    loop {
        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => menu.previous(),
            NavAction::Down => menu.next(),
            NavAction::Select => match menu.selected_index() {
                Some(0) => return Ok(false),
                Some(1) => return Ok(true),
                _ => {}
            },
            NavAction::ForceQuit | NavAction::Back => return Ok(false),
            NavAction::None => {}
        }
    }
}

/// Show a message in TUI
pub fn run_message_tui(
    terminal: &mut Tui,
    message: &str,
    color: Color,
) -> Result<(), Box<dyn Error>> {
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

            let para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

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
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

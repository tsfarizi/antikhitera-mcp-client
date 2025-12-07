//! Prompt template editing TUI

use super::load_config;
use crate::config::wizard::generator;
use crate::tui::terminal::{NavAction, Tui, read_key};
use crate::tui::widgets::{Menu, MenuItem};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::error::Error;

/// Default prompt template
const DEFAULT_TEMPLATE: &str = r#"You are a helpful AI assistant.

{{custom_instruction}}

{{language_guidance}}

{{tool_guidance}}"#;

/// Edit prompt template screen (uses existing terminal)
pub fn run_edit_prompt_with_terminal(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    let mut selected_idx: usize = 0;

    loop {
        let config = load_config()?;

        // Truncate template preview
        let preview: String = config
            .prompt_template
            .lines()
            .take(2)
            .collect::<Vec<_>>()
            .join(" | ");
        let preview_short = if preview.len() > 40 {
            format!("{}...", preview.chars().take(40).collect::<String>())
        } else {
            preview
        };

        let items = vec![
            MenuItem::new("🔄 Reset to Default"),
            MenuItem::new("✏️  Edit Template"),
            MenuItem::new("👁️  View Current Template"),
            MenuItem::new("← Back"),
        ];

        let mut menu = Menu::new("📝 Edit Prompt Template", items)
            .with_subtitle(format!("Current: {}", preview_short));
        menu.select(selected_idx);

        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => {
                menu.previous();
                selected_idx = menu.selected_index().unwrap_or(0);
            }
            NavAction::Down => {
                menu.next();
                selected_idx = menu.selected_index().unwrap_or(0);
            }
            NavAction::Select => {
                match menu.selected_index() {
                    Some(0) => {
                        // Reset to Default - with double confirmation in TUI
                        if run_reset_confirmation_tui(terminal)? {
                            generator::update_prompt_template(DEFAULT_TEMPLATE)?;
                            run_message_tui(
                                terminal,
                                "✓ Template reset to default!",
                                Color::Green,
                            )?;
                        }
                    }
                    Some(1) => {
                        // Edit Template - show editor in TUI
                        run_edit_template_tui(terminal, &config.prompt_template)?;
                    }
                    Some(2) => {
                        // View Current Template in TUI
                        run_view_template_tui(terminal, &config.prompt_template)?;
                    }
                    Some(3) => break,
                    _ => {}
                }
            }
            NavAction::ForceQuit | NavAction::Back => break,
            NavAction::None => {}
        }
    }

    Ok(())
}

/// Show a message in TUI
fn run_message_tui(
    terminal: &mut crate::tui::terminal::Tui,
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

/// Double confirmation for reset in TUI
fn run_reset_confirmation_tui(
    terminal: &mut crate::tui::terminal::Tui,
) -> Result<bool, Box<dyn Error>> {
    // First confirmation
    let items = vec![
        MenuItem::new("❌ Cancel"),
        MenuItem::new("⚠️  Yes, Reset Template"),
    ];
    let mut menu = Menu::new("⚠️  Reset Prompt Template?", items)
        .with_subtitle("This will replace your current template with the default.");

    loop {
        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => menu.previous(),
            NavAction::Down => menu.next(),
            NavAction::Select => {
                match menu.selected_index() {
                    Some(0) => return Ok(false), // Cancel
                    Some(1) => break,            // Proceed to second confirmation
                    _ => {}
                }
            }
            NavAction::ForceQuit | NavAction::Back => return Ok(false),
            NavAction::None => {}
        }
    }

    // Second confirmation
    let items2 = vec![
        MenuItem::new("❌ Cancel - Keep Current Template"),
        MenuItem::new("🔥 CONFIRM RESET"),
    ];
    let mut menu2 = Menu::new("🔥 Final Confirmation", items2)
        .with_subtitle("Are you absolutely sure? This cannot be undone!");

    loop {
        terminal.draw(|frame| {
            menu2.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => menu2.previous(),
            NavAction::Down => menu2.next(),
            NavAction::Select => {
                match menu2.selected_index() {
                    Some(0) => return Ok(false), // Cancel
                    Some(1) => return Ok(true),  // Confirm
                    _ => {}
                }
            }
            NavAction::ForceQuit | NavAction::Back => return Ok(false),
            NavAction::None => {}
        }
    }
}

/// View template in TUI
fn run_view_template_tui(
    terminal: &mut crate::tui::terminal::Tui,
    template: &str,
) -> Result<(), Box<dyn Error>> {
    let mut scroll: u16 = 0;

    // Build lines with simple formatting
    let template_lines: Vec<&str> = template.lines().collect();
    let total_lines = template_lines.len() as u16;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // Build display lines - plain text, no line numbers
            let display_lines: Vec<Line> = template_lines
                .iter()
                .map(|line| Line::from(*line))
                .collect();

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Current Template ")
                .title_bottom(format!(
                    " [Up/Down: Scroll] [ESC: Back] Lines: {} ",
                    total_lines
                ));

            let para = Paragraph::new(display_lines)
                .block(block)
                .scroll((scroll, 0))
                .wrap(Wrap { trim: true });

            frame.render_widget(para, area);
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => {
                scroll = scroll.saturating_sub(1);
            }
            NavAction::Down => {
                if scroll < total_lines.saturating_sub(3) {
                    scroll += 1;
                }
            }
            NavAction::Select | NavAction::Back | NavAction::ForceQuit => break,
            NavAction::None => {}
        }
    }
    Ok(())
}

/// Edit template in TUI with direct typing
fn run_edit_template_tui(
    terminal: &mut crate::tui::terminal::Tui,
    current: &str,
) -> Result<(), Box<dyn Error>> {
    use crossterm::event::{self, Event, KeyCode, KeyModifiers};

    // Start with current template content
    let mut lines: Vec<String> = current.lines().map(|s| s.to_string()).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }

    let mut cursor_row: usize = 0;
    let mut cursor_col: usize = 0;
    let mut scroll: u16 = 0;

    loop {
        // Ensure cursor is within bounds
        cursor_row = cursor_row.min(lines.len().saturating_sub(1));
        cursor_col = cursor_col.min(lines.get(cursor_row).map(|l| l.len()).unwrap_or(0));

        terminal.draw(|frame| {
            let area = frame.area();

            // Build display with cursor indicator
            let display_lines: Vec<Line> = lines
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    if i == cursor_row {
                        // Show cursor position with underscore or pipe
                        let mut display = line.clone();
                        if cursor_col >= display.len() {
                            display.push('_');
                        } else {
                            // Insert cursor marker
                            let mut chars: Vec<char> = display.chars().collect();
                            chars.insert(cursor_col, '|');
                            display = chars.into_iter().collect();
                        }
                        Line::from(Span::styled(display, Style::default().fg(Color::Yellow)))
                    } else {
                        Line::from(line.as_str())
                    }
                })
                .collect();

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(" Edit Template (Type to edit) ")
                .title_bottom(" [Ctrl+S: Save] [ESC: Cancel] [Enter: New line] ");

            let para = Paragraph::new(display_lines)
                .block(block)
                .scroll((scroll, 0))
                .wrap(Wrap { trim: false });

            frame.render_widget(para, area);
        })?;

        // Read raw key events for typing
        if let Event::Key(key) = event::read()? {
            if key.kind != crossterm::event::KeyEventKind::Press {
                continue;
            }

            // Handle Ctrl+S to save
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                let new_template = lines.join("\n");
                generator::update_prompt_template(&new_template)?;
                break;
            }

            // Handle Ctrl+Q to force quit
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
                break;
            }

            match key.code {
                KeyCode::Esc => break, // Cancel without saving
                KeyCode::Enter => {
                    // Split line at cursor and insert new line
                    let current_line = lines.get(cursor_row).cloned().unwrap_or_default();
                    let (before, after) = current_line.split_at(cursor_col.min(current_line.len()));
                    lines[cursor_row] = before.to_string();
                    lines.insert(cursor_row + 1, after.to_string());
                    cursor_row += 1;
                    cursor_col = 0;
                }
                KeyCode::Backspace => {
                    if cursor_col > 0 {
                        // Delete char before cursor
                        if let Some(line) = lines.get_mut(cursor_row) {
                            let mut chars: Vec<char> = line.chars().collect();
                            if cursor_col <= chars.len() {
                                chars.remove(cursor_col - 1);
                                *line = chars.into_iter().collect();
                                cursor_col -= 1;
                            }
                        }
                    } else if cursor_row > 0 {
                        // Merge with previous line
                        let current_line = lines.remove(cursor_row);
                        cursor_row -= 1;
                        cursor_col = lines[cursor_row].len();
                        lines[cursor_row].push_str(&current_line);
                    }
                }
                KeyCode::Delete => {
                    if let Some(line) = lines.get_mut(cursor_row) {
                        let chars: Vec<char> = line.chars().collect();
                        if cursor_col < chars.len() {
                            let mut chars = chars;
                            chars.remove(cursor_col);
                            *line = chars.into_iter().collect();
                        } else if cursor_row + 1 < lines.len() {
                            // Merge with next line
                            let next_line = lines.remove(cursor_row + 1);
                            lines[cursor_row].push_str(&next_line);
                        }
                    }
                }
                KeyCode::Left => {
                    if cursor_col > 0 {
                        cursor_col -= 1;
                    } else if cursor_row > 0 {
                        cursor_row -= 1;
                        cursor_col = lines.get(cursor_row).map(|l| l.len()).unwrap_or(0);
                    }
                }
                KeyCode::Right => {
                    let line_len = lines.get(cursor_row).map(|l| l.len()).unwrap_or(0);
                    if cursor_col < line_len {
                        cursor_col += 1;
                    } else if cursor_row + 1 < lines.len() {
                        cursor_row += 1;
                        cursor_col = 0;
                    }
                }
                KeyCode::Up => {
                    if cursor_row > 0 {
                        cursor_row -= 1;
                        if scroll > 0 && cursor_row < scroll as usize {
                            scroll -= 1;
                        }
                    }
                }
                KeyCode::Down => {
                    if cursor_row + 1 < lines.len() {
                        cursor_row += 1;
                    }
                }
                KeyCode::Home => {
                    cursor_col = 0;
                }
                KeyCode::End => {
                    cursor_col = lines.get(cursor_row).map(|l| l.len()).unwrap_or(0);
                }
                KeyCode::Char(c) => {
                    // Insert character at cursor
                    if let Some(line) = lines.get_mut(cursor_row) {
                        let mut chars: Vec<char> = line.chars().collect();
                        chars.insert(cursor_col.min(chars.len()), c);
                        *line = chars.into_iter().collect();
                        cursor_col += 1;
                    }
                }
                _ => {}
            }
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

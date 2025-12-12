//! Editor widgets for prompts
//!
//! Provides multiline and singleline editors for prompt fields.

use crate::tui::terminal::Tui;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::error::Error;

use super::{centered_rect, update_field};

/// Edit multiline field (like template)
pub fn run_edit_multiline(
    terminal: &mut Tui,
    field_name: &str,
    current: &str,
) -> Result<(), Box<dyn Error>> {
    let mut lines: Vec<String> = current.lines().map(|s| s.to_string()).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }

    let mut cursor_row: usize = 0;
    let mut cursor_col: usize = 0;
    let mut scroll: u16 = 0;

    loop {
        cursor_row = cursor_row.min(lines.len().saturating_sub(1));
        cursor_col = cursor_col.min(lines.get(cursor_row).map(|l| l.len()).unwrap_or(0));

        terminal.draw(|frame| {
            let area = frame.area();
            let display_lines: Vec<Line> = lines
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    if i == cursor_row {
                        let mut display = line.clone();
                        if cursor_col >= display.len() {
                            display.push('_');
                        } else {
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
                .title(format!(" Edit {} (Type to edit) ", field_name))
                .title_bottom(" [Ctrl+S: Save] [ESC: Cancel] [Enter: New line] ");

            let para = Paragraph::new(display_lines)
                .block(block)
                .scroll((scroll, 0))
                .wrap(Wrap { trim: false });

            frame.render_widget(para, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != crossterm::event::KeyEventKind::Press {
                continue;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                let new_value = lines.join("\n");
                update_field(field_name, &new_value)?;
                break;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
                break;
            }

            match key.code {
                KeyCode::Esc => break,
                KeyCode::Enter => {
                    let current_line = lines.get(cursor_row).cloned().unwrap_or_default();
                    let (before, after) = current_line.split_at(cursor_col.min(current_line.len()));
                    lines[cursor_row] = before.to_string();
                    lines.insert(cursor_row + 1, after.to_string());
                    cursor_row += 1;
                    cursor_col = 0;
                }
                KeyCode::Backspace => {
                    if cursor_col > 0 {
                        if let Some(line) = lines.get_mut(cursor_row) {
                            let mut chars: Vec<char> = line.chars().collect();
                            if cursor_col <= chars.len() {
                                chars.remove(cursor_col - 1);
                                *line = chars.into_iter().collect();
                                cursor_col -= 1;
                            }
                        }
                    } else if cursor_row > 0 {
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
                KeyCode::Home => cursor_col = 0,
                KeyCode::End => {
                    cursor_col = lines.get(cursor_row).map(|l| l.len()).unwrap_or(0);
                }
                KeyCode::Char(c) => {
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

/// Edit single-line field
pub fn run_edit_singleline(
    terminal: &mut Tui,
    field_name: &str,
    label: &str,
    current: &str,
) -> Result<(), Box<dyn Error>> {
    let mut text = current.to_string();
    let mut cursor: usize = text.len();

    loop {
        terminal.draw(|frame| {
            let area = centered_rect(80, 30, frame.area());
            frame.render_widget(Clear, area);

            let mut display = text.clone();
            if cursor >= display.len() {
                display.push('_');
            } else {
                let mut chars: Vec<char> = display.chars().collect();
                chars.insert(cursor, '|');
                display = chars.into_iter().collect();
            }

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(format!(" Edit {} ", label))
                .title_bottom(" [Ctrl+S: Save] [ESC: Cancel] ");

            let para = Paragraph::new(display)
                .block(block)
                .wrap(Wrap { trim: false });

            frame.render_widget(para, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != crossterm::event::KeyEventKind::Press {
                continue;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                update_field(field_name, &text)?;
                break;
            }

            match key.code {
                KeyCode::Esc => break,
                KeyCode::Backspace => {
                    if cursor > 0 {
                        let mut chars: Vec<char> = text.chars().collect();
                        chars.remove(cursor - 1);
                        text = chars.into_iter().collect();
                        cursor -= 1;
                    }
                }
                KeyCode::Delete => {
                    let chars: Vec<char> = text.chars().collect();
                    if cursor < chars.len() {
                        let mut chars = chars;
                        chars.remove(cursor);
                        text = chars.into_iter().collect();
                    }
                }
                KeyCode::Left => {
                    if cursor > 0 {
                        cursor -= 1;
                    }
                }
                KeyCode::Right => {
                    if cursor < text.len() {
                        cursor += 1;
                    }
                }
                KeyCode::Home => cursor = 0,
                KeyCode::End => cursor = text.len(),
                KeyCode::Char(c) => {
                    let mut chars: Vec<char> = text.chars().collect();
                    chars.insert(cursor.min(chars.len()), c);
                    text = chars.into_iter().collect();
                    cursor += 1;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

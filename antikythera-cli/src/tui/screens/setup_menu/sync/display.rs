//! Display functions for sync progress
//!
//! Provides TUI widgets for showing sync status and results.

use crate::tui::terminal::{NavAction, Tui, read_key};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::error::Error;

/// Show status during sync
pub fn show_status_tui(
    terminal: &mut Tui,
    title: &str,
    status: &str,
) -> Result<(), Box<dyn Error>> {
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
pub fn show_result_tui(
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
pub fn show_all_results_tui(
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

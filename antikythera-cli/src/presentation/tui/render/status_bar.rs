//! Footer status bar showing health indicator and keyboard shortcut hints.

use antikythera_core::application::resilience::HealthStatus;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::app::ChatApp;

pub(super) fn draw_status_bar(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let (health_dot, health_color) = match app.health.try_lock() {
        Ok(h) => match h.overall_status() {
            HealthStatus::Healthy => ("\u{25cf} ", Color::Green),
            HealthStatus::Degraded => ("\u{25cf} ", Color::Yellow),
            HealthStatus::Unhealthy => ("\u{25cf} ", Color::Red),
        },
        Err(_) => ("\u{25cb} ", Color::Gray),
    };
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            health_dot,
            Style::default()
                .fg(health_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" autocomplete  "),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" submit  "),
        Span::styled(
            "F2",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" settings  "),
        Span::styled(
            "F3",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" riwayat  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" quit  "),
        Span::styled(
            "↑↓",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" scroll chat  "),
        Span::styled(
            "Ctrl+↑↓",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" scroll log  "),
        Span::styled(app.status.as_str(), Style::default().fg(Color::Gray)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(footer, area);
}

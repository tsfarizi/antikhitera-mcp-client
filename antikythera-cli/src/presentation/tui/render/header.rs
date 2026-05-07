//! Header bar showing app title, provider/model, and mode.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::app::ChatApp;

pub(super) fn draw_header(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " Antikythera CLI ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} / {}", app.provider, app.model),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            if app.agent_mode {
                "Agent Loop"
            } else {
                "Direct Chat"
            },
            Style::default().fg(Color::Yellow),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Session"));
    frame.render_widget(header, area);
}

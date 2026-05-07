use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use super::super::app::ChatApp;

pub(super) fn draw_settings_tab_provider(
    frame: &mut ratatui::Frame<'_>,
    app: &ChatApp,
    area: Rect,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let items: Vec<ListItem> = app
        .providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let selected = i == app.settings.pending_provider_idx;
            let cursor = i == app.settings.provider_cursor;
            let radio = if selected { "◉" } else { "○" };
            let arrow = if cursor { "▶" } else { " " };
            let style = if cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            ListItem::new(format!(
                "{arrow} {radio} {:<18} [{}]",
                p.id, p.provider_type
            ))
            .style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Provider  [↑↓=navigasi | Enter=pilih & ke tab Model]")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, cols[0]);

    if let Some(p) = app.providers.get(app.settings.provider_cursor) {
        let models_text = p
            .models
            .iter()
            .map(|m| format!("  • {}", m.display_name.as_deref().unwrap_or(&m.name)))
            .collect::<Vec<_>>()
            .join("\n");
        let api_status = if p.api_key.is_some() {
            "✓ tersedia"
        } else {
            "✗ tidak ada (pakai env var)"
        };
        let detail = format!(
            "ID       : {}\nType     : {}\nEndpoint : {}\nAPI Key  : {}\n\nModels:\n{}",
            p.id,
            p.provider_type,
            p.endpoint,
            api_status,
            if models_text.is_empty() {
                "  (tidak ada model terdaftar)".to_string()
            } else {
                models_text
            }
        );
        let widget = Paragraph::new(detail)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Detail Provider"),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(widget, cols[1]);
    }
}

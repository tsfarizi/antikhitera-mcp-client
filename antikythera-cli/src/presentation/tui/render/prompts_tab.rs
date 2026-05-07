use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use super::super::app::ChatApp;
use super::super::types::PromptField;

pub(super) fn draw_settings_tab_prompts(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let items: Vec<ListItem> = PromptField::ALL
        .iter()
        .enumerate()
        .map(|(i, &field)| {
            let cursor = i == app.settings.prompt_cursor;
            let arrow = if cursor { "▶" } else { " " };
            let preview = field
                .get_from(&app.settings.pending_prompts)
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(58)
                .collect::<String>();
            let style = if cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{arrow} {:<22} {}", field.label(), preview)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(
                "Prompt Fields  [↑↓=pilih | Enter=edit | Ctrl+Enter=simpan field | Esc=batal edit]",
            )
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, rows[0]);

    if app.settings.editing {
        let edit_widget = Paragraph::new(app.settings.edit_buffer.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Edit Field  [Ctrl+Enter=simpan | Esc=batal | Enter=baris baru]")
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(edit_widget, rows[1]);
    } else if let Some(&field) = PromptField::ALL.get(app.settings.prompt_cursor) {
        let preview_text = field.get_from(&app.settings.pending_prompts);
        let preview_widget = Paragraph::new(preview_text.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Preview: {}  [Enter=edit]", field.label()))
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(preview_widget, rows[1]);
    }
}

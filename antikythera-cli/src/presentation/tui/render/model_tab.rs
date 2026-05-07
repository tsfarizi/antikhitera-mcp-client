use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use super::super::app::ChatApp;

pub(super) fn draw_settings_tab_model(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let (list_area, input_area) = if app.settings.model_add_mode {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(list_area);

    let Some(provider) = app.providers.get(app.settings.pending_provider_idx) else {
        let msg = Paragraph::new("Pilih provider terlebih dahulu di tab [1] Provider.")
            .block(Block::default().borders(Borders::ALL).title("Model"));
        frame.render_widget(msg, area);
        return;
    };

    let items: Vec<ListItem> = provider
        .models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let selected = i == app.settings.pending_model_idx;
            let cursor = i == app.settings.model_cursor;
            let radio = if selected { "\u{25c9}" } else { "\u{25cb}" };
            let arrow = if cursor { "\u{25b6}" } else { " " };
            let style = if cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            let name = m.display_name.as_deref().unwrap_or(&m.name);
            ListItem::new(format!("{arrow} {radio} {name}")).style(style)
        })
        .collect();

    let list_title = format!(
        "Model '{}' [\u{2191}\u{2193}=navigasi | Enter=pilih | a=tambah | d=hapus]",
        provider.id
    );
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(list_title)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, cols[0]);

    if provider.models.is_empty() {
        let hint = Paragraph::new(
            "Belum ada model.\n\nTekan [a] untuk menambahkan nama model\n(contoh: gemini-2.0-flash)",
        )
        .block(Block::default().borders(Borders::ALL).title("Detail Model"))
        .wrap(Wrap { trim: false });
        frame.render_widget(hint, cols[1]);
    } else if let Some(m) = provider.models.get(app.settings.model_cursor) {
        let status = if app.settings.model_cursor == app.settings.pending_model_idx {
            "\u{25c9} terpilih"
        } else {
            "\u{25cb} belum dipilih (tekan Enter untuk memilih)"
        };
        let detail = format!(
            "Name         : {}\nDisplay Name : {}\n\nStatus       : {}",
            m.name,
            m.display_name.as_deref().unwrap_or("(sama dengan name)"),
            status
        );
        let widget = Paragraph::new(detail)
            .block(Block::default().borders(Borders::ALL).title("Detail Model"))
            .wrap(Wrap { trim: false });
        frame.render_widget(widget, cols[1]);
    }

    if let Some(input_rect) = input_area {
        let prompt = format!("Nama model: {}\u{2588}", app.settings.model_add_buffer);
        let input_widget = Paragraph::new(prompt)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Tambah Model  [Enter=simpan | Esc=batal]")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(input_widget, input_rect);
    }
}

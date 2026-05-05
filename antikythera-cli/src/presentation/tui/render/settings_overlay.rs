//! Settings overlay with Provider, Model, Prompts, System, Agent tabs.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use super::super::app::ChatApp;
use super::super::types::{PromptField, SettingsTab};

pub(super) fn draw_settings_overlay(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
    let area = frame.area();
    // Blank the entire terminal before drawing the overlay.
    frame.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚙  Settings  [Tab/BackTab=ganti tab | ↑↓=nav | Enter=pilih | Ctrl+S=simpan | Esc=tutup] ")
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .split(inner);

    // Tab bar — highlight the active tab.
    let mut tab_spans: Vec<Span> = Vec::new();
    for (i, tab) in SettingsTab::ALL.iter().enumerate() {
        let label = format!(" [{}] {} ", i + 1, tab.label());
        if *tab == app.settings.tab {
            tab_spans.push(Span::styled(
                label,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            tab_spans.push(Span::styled(label, Style::default().fg(Color::Cyan)));
        }
        tab_spans.push(Span::raw("  "));
    }
    let tab_bar =
        Paragraph::new(Line::from(tab_spans)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(tab_bar, layout[0]);

    // Render the content area for the active tab.
    match app.settings.tab {
        SettingsTab::Provider => draw_settings_tab_provider(frame, app, layout[1]),
        SettingsTab::Model => draw_settings_tab_model(frame, app, layout[1]),
        SettingsTab::Prompts => draw_settings_tab_prompts(frame, app, layout[1]),
        SettingsTab::System => draw_settings_tab_system(frame, app, layout[1]),
        SettingsTab::Agent => draw_settings_tab_agent(frame, app, layout[1]),
    }
}

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

pub(super) fn draw_settings_tab_model(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    // Reserve the bottom row for the add-model input bar when active.
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
            let name = m.display_name.as_deref().unwrap_or(&m.name);
            ListItem::new(format!("{arrow} {radio} {name}")).style(style)
        })
        .collect();

    let list_title = format!(
        "Model '{}' [↑↓=navigasi | Enter=pilih | a=tambah | d=hapus]",
        provider.id
    );
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(list_title)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, cols[0]);

    // Right column: detail or empty state hint.
    if provider.models.is_empty() {
        let hint = Paragraph::new(
            "Belum ada model.\n\nTekan [a] untuk menambahkan nama model\n(contoh: gemini-2.0-flash)",
        )
        .block(Block::default().borders(Borders::ALL).title("Detail Model"))
        .wrap(Wrap { trim: false });
        frame.render_widget(hint, cols[1]);
    } else if let Some(m) = provider.models.get(app.settings.model_cursor) {
        let status = if app.settings.model_cursor == app.settings.pending_model_idx {
            "◉ terpilih"
        } else {
            "○ belum dipilih (tekan Enter untuk memilih)"
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

    // Add-model input bar at the bottom.
    if let Some(input_rect) = input_area {
        let prompt = format!("Nama model: {}█", app.settings.model_add_buffer);
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

pub(super) fn draw_settings_tab_system(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(8)])
        .split(area);

    let info = Paragraph::new(
        "System prompt di-inject ke setiap sesi baru sebagai instruksi dasar.\n\
         Biarkan kosong untuk menggunakan template default dari PromptsConfig.\n\
         Tekan Enter untuk mulai edit. Ctrl+Enter untuk simpan perubahan.",
    )
    .block(Block::default().borders(Borders::ALL).title("Info"))
    .wrap(Wrap { trim: false });
    frame.render_widget(info, rows[0]);

    if app.settings.editing {
        let edit = Paragraph::new(app.settings.edit_buffer.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Edit System Prompt  [Ctrl+Enter=simpan | Esc=batal | Enter=baris baru]")
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(edit, rows[1]);
    } else {
        let current = if app.settings.pending_system_prompt.is_empty() {
            "(kosong — menggunakan template default dari PromptsConfig)".to_string()
        } else {
            app.settings.pending_system_prompt.clone()
        };
        let preview = Paragraph::new(current.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("System Prompt Aktif  [Enter=edit]")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(preview, rows[1]);
    }
}

pub(super) fn draw_settings_tab_agent(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(4)])
        .split(area);

    let mode_label = if app.settings.pending_agent_mode {
        "◉ Agent Loop  (aktif)"
    } else {
        "○ Direct Chat (aktif)"
    };
    let content = format!(
        "Mode Eksekusi  : {}\n\n\
         Tekan Enter untuk toggle mode.\n\n\
         ◉ Agent Loop   — Prompt dieksekusi melalui planning loop & tool calls.\n\
         ○ Direct Chat  — Prompt langsung dikirim ke model tanpa loop agent.\n\n\
         Ctrl+S untuk menyimpan semua perubahan settings.",
        mode_label
    );
    let widget = Paragraph::new(content.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Agent Settings  [Enter=toggle | Ctrl+S=simpan semua]")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, rows[0]);

    // Quick summary of all pending changes.
    let provider_name = app
        .providers
        .get(app.settings.pending_provider_idx)
        .map(|p| p.id.as_str())
        .unwrap_or("(tidak ada)");
    let model_name = app
        .providers
        .get(app.settings.pending_provider_idx)
        .and_then(|p| p.models.get(app.settings.pending_model_idx))
        .map(|m| m.name.as_str())
        .unwrap_or("(tidak ada)");
    let summary = format!(
        "Pending Changes:\n  Provider     : {}\n  Model        : {}\n  Mode         : {}\n  System Prompt: {} karakter",
        provider_name,
        model_name,
        if app.settings.pending_agent_mode {
            "Agent Loop"
        } else {
            "Direct Chat"
        },
        app.settings.pending_system_prompt.len(),
    );
    let summary_widget = Paragraph::new(summary.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Ringkasan Perubahan Pending  [Ctrl+S=terapkan semua]")
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(summary_widget, rows[1]);
}

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::super::app::ChatApp;

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

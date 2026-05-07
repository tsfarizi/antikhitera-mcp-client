use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::super::app::ChatApp;

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

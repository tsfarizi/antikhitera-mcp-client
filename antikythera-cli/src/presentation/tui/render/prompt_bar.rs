//! Prompt input bar at the bottom of the chat area.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::app::ChatApp;

pub(super) fn draw_prompt_bar(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let prompt_title = if app.loading {
        "Prompt  [mengirim...]"
    } else {
        "Prompt  [F2 = Settings | F3 = Riwayat | Enter = kirim | /help = commands]"
    };
    let input_widget = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(prompt_title))
        .style(if app.loading {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    frame.render_widget(input_widget, area);
}

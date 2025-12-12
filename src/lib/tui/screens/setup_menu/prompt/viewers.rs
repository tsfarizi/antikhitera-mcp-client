//! Viewer widgets for prompts
//!
//! Provides scrollable content viewers for prompt fields.

use crate::config::PromptsConfig;
use crate::tui::terminal::{NavAction, Tui, read_key};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::error::Error;

use super::{PROMPT_FIELDS, get_field_value};

/// View content (scrollable)
pub fn run_view_content(
    terminal: &mut Tui,
    title: &str,
    content: &str,
) -> Result<(), Box<dyn Error>> {
    let mut scroll: u16 = 0;
    let content_lines: Vec<&str> = content.lines().collect();
    let total_lines = content_lines.len() as u16;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let display_lines: Vec<Line> =
                content_lines.iter().map(|line| Line::from(*line)).collect();

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" {} ", title))
                .title_bottom(format!(
                    " [Up/Down: Scroll] [ESC: Back] Lines: {} ",
                    total_lines
                ));

            let para = Paragraph::new(display_lines)
                .block(block)
                .scroll((scroll, 0))
                .wrap(Wrap { trim: true });

            frame.render_widget(para, area);
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => scroll = scroll.saturating_sub(1),
            NavAction::Down => {
                if scroll < total_lines.saturating_sub(3) {
                    scroll += 1;
                }
            }
            NavAction::Select | NavAction::Back | NavAction::ForceQuit => break,
            NavAction::None => {}
        }
    }
    Ok(())
}

/// View all prompts
pub fn run_view_all_prompts(
    terminal: &mut Tui,
    prompts: &PromptsConfig,
) -> Result<(), Box<dyn Error>> {
    let mut content = String::new();
    for field in PROMPT_FIELDS {
        content.push_str(&format!("=== {} {} ===\n", field.icon, field.label));
        content.push_str(&get_field_value(prompts, field.name));
        content.push_str("\n\n");
    }
    run_view_content(terminal, "All Prompts", &content)
}

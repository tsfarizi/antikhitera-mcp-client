//! TUI render orchestrator — composes chat, log, settings, and history panels.

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::app::ChatApp;

pub(crate) mod agent_tab;
mod autocomplete;
mod conversation;
mod header;
pub(crate) mod history_overlay;
pub mod log_panel;
pub(crate) mod model_tab;
mod prompt_bar;
pub(crate) mod prompts_tab;
pub(crate) mod provider_tab;
pub(crate) mod settings_overlay;
mod sidebar;
mod status_bar;
pub(crate) mod system_tab;

pub(super) fn draw(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(16),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(layout[1]);

    let right_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(content[1]);

    // Header
    header::draw_header(frame, app, layout[0]);

    // Conversation
    conversation::draw_conversation(frame, app, content[0]);

    // Context sidebar
    sidebar::draw_sidebar(frame, app, right_panel[0]);

    // WASM / FFI log panel
    let log_panel_area = right_panel[1];
    let all_log_lines: Vec<&String> = app.log_lines.iter().collect();
    let log_styled_lines: Vec<Line<'_>> = all_log_lines
        .iter()
        .map(|line| {
            let style = log_panel::resolve_log_line_style(line);
            Line::from(Span::styled(line.as_str(), style))
        })
        .collect();
    let log_text = Text::from(log_styled_lines);
    let log_panel = Paragraph::new(log_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Logs [yellow=CLI/prov | magenta=SDK/FFI | cyan=stream | green=agent | blue=tool/transport] [Ctrl+↑↓/PgUp/PgDn/Home/End = scroll]")
                .title_style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .wrap(Wrap { trim: true })
        .scroll((app.log_scroll, 0));
    frame.render_widget(log_panel, log_panel_area);

    // Prompt bar
    prompt_bar::draw_prompt_bar(frame, app, layout[2]);

    // Command autocomplete overlay
    let autocomplete_area = centered_rect(72, 34, frame.area());
    autocomplete::draw_autocomplete(frame, app, autocomplete_area);

    // Footer / status
    status_bar::draw_status_bar(frame, app, layout[3]);

    // Settings overlay (drawn on top of everything else)
    if app.settings.open {
        settings_overlay::draw_settings_overlay(frame, app);
    }

    // History overlay (drawn on top of everything else)
    if app.history.open {
        history_overlay::draw_history_overlay(frame, app);
    }
}

fn centered_rect(
    horizontal_percent: u16,
    vertical_percent: u16,
    area: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - vertical_percent) / 2),
            Constraint::Percentage(vertical_percent),
            Constraint::Percentage((100 - vertical_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - horizontal_percent) / 2),
            Constraint::Percentage(horizontal_percent),
            Constraint::Percentage((100 - horizontal_percent) / 2),
        ])
        .split(vertical[1])[1]
}

//! Chat UI rendering components

use super::state::{ChatState, MessageRole};
use crate::tui::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Main chat UI renderer
pub struct ChatUI;

impl ChatUI {
    /// Render the complete chat interface
    pub fn render(frame: &mut Frame, state: &ChatState, provider: &str, model: &str) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Status bar
                Constraint::Min(5),    // Messages area
                Constraint::Length(3), // Input area
                Constraint::Length(1), // Help bar
            ])
            .split(area);

        Self::render_status_bar(frame, chunks[0], state, provider, model);
        Self::render_messages(frame, chunks[1], state);
        Self::render_input(frame, chunks[2], state);
        Self::render_help_bar(frame, chunks[3], state);
    }

    /// Render status bar with session info
    fn render_status_bar(
        frame: &mut Frame,
        area: Rect,
        state: &ChatState,
        provider: &str,
        model: &str,
    ) {
        let session_display = state
            .session_id
            .as_ref()
            .map(|s| s.chars().take(8).collect::<String>())
            .unwrap_or_else(|| "new".into());

        let mode_indicator = if state.agent_mode {
            Span::styled(" Agent ", theme::mode_agent())
        } else {
            Span::styled(" Chat ", theme::mode_chat())
        };

        let loading_indicator = if state.loading {
            let frames = ["⠋", "⠙", "⠹", "⠸"];
            Span::styled(
                format!(" {} ", frames[state.loading_frame]),
                theme::loading(),
            )
        } else {
            Span::raw("")
        };

        let status_msg = state
            .status_message
            .as_ref()
            .map(|s| Span::styled(format!(" │ {} ", s), theme::subtitle()))
            .unwrap_or_else(|| Span::raw(""));

        let status_line = Line::from(vec![
            Span::styled(" 💬 ", theme::title()),
            Span::styled(format!("Session: {} ", session_display), theme::text()),
            Span::styled("│ ", theme::subtitle()),
            mode_indicator,
            Span::styled(" │ ", theme::subtitle()),
            Span::styled(
                format!("{}/{}", provider, model),
                Style::default().fg(theme::HIGHLIGHT),
            ),
            loading_indicator,
            status_msg,
        ]);

        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme::border());

        let para = Paragraph::new(status_line).block(block);
        frame.render_widget(para, area);
    }

    /// Render messages area
    fn render_messages(frame: &mut Frame, area: Rect, state: &ChatState) {
        let inner_height = area.height.saturating_sub(2) as usize;
        let mut lines: Vec<Line> = Vec::new();

        for msg in &state.messages {
            let (prefix, style) = match msg.role {
                MessageRole::User => ("You: ", theme::user_prefix()),
                MessageRole::Assistant => ("AI: ", theme::ai_prefix()),
                MessageRole::System => ("System: ", theme::system_prefix()),
            };
            let content_lines: Vec<&str> = msg.content.lines().collect();
            if let Some(first_line) = content_lines.first() {
                lines.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::raw(*first_line),
                ]));
            }
            for line in content_lines.iter().skip(1) {
                let indent = " ".repeat(prefix.len());
                lines.push(Line::from(format!("{}{}", indent, line)));
            }
            lines.push(Line::from(""));
        }
        if state.loading {
            let frames = ["⠋", "⠙", "⠹", "⠸"];
            lines.push(Line::from(Span::styled(
                format!("AI: {} Thinking...", frames[state.loading_frame]),
                theme::loading(),
            )));
        }
        let total_lines = lines.len();
        let max_scroll = total_lines.saturating_sub(inner_height);
        let scroll = if state.scroll_offset == u16::MAX {
            max_scroll as u16
        } else {
            state.scroll_offset.min(max_scroll as u16)
        };

        let block = Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(theme::border());

        let para = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        frame.render_widget(para, area);
    }

    /// Render input area
    fn render_input(frame: &mut Frame, area: Rect, state: &ChatState) {
        let input_style = if state.loading {
            theme::subtitle()
        } else {
            theme::text()
        };
        let display_input = if state.loading {
            "Waiting for response...".to_string()
        } else if state.input.is_empty() {
            "Type your message...".to_string()
        } else {
            let mut chars: Vec<char> = state.input.chars().collect();
            if state.cursor_pos >= chars.len() {
                chars.push('_');
            } else {
                chars.insert(state.cursor_pos, '|');
            }
            chars.into_iter().collect()
        };

        let input_line = Line::from(vec![
            Span::styled("> ", theme::title()),
            Span::styled(display_input, input_style),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if state.loading {
                theme::border()
            } else {
                theme::border_active()
            })
            .title(if state.is_command() {
                " Command "
            } else {
                " Message "
            });

        let para = Paragraph::new(input_line).block(block);
        frame.render_widget(para, area);
    }

    /// Render help bar
    fn render_help_bar(frame: &mut Frame, area: Rect, state: &ChatState) {
        let help_text = if state.loading {
            Line::from(Span::styled(
                " Processing... Please wait ",
                theme::loading(),
            ))
        } else {
            Line::from(vec![
                Span::styled(" Enter", theme::key_hint()),
                Span::raw(": Send │ "),
                Span::styled("/help", theme::key_hint()),
                Span::raw(": Commands │ "),
                Span::styled("PageUp/Down", theme::key_hint()),
                Span::raw(": Scroll │ "),
                Span::styled("q", theme::key_destructive()),
                Span::raw(": Exit "),
            ])
        };

        let para = Paragraph::new(help_text);
        frame.render_widget(para, area);
    }
}

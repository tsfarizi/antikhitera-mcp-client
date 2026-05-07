//! Conversation panel — message list with streaming preview.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::super::app::ChatApp;
use super::super::types::{UiMessage, UiTone};

pub(super) fn draw_conversation(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let conv_title = if app.loading {
        if app.streaming_content.is_empty() {
            "Conversation  [mengirim...]"
        } else {
            "Conversation  [menerima...]"
        }
    } else {
        "Conversation  [↑↓/PgUp/PgDn/Home/End = scroll]"
    };
    let mut conv_text = render_messages(app.messages.iter());
    if app.loading && !app.streaming_content.is_empty() {
        conv_text.extend(render_streaming_preview(&app.streaming_content));
    }
    let conversation = Paragraph::new(conv_text)
        .block(Block::default().borders(Borders::ALL).title(conv_title))
        .wrap(Wrap { trim: false })
        .scroll((app.conversation_scroll, 0));
    frame.render_widget(conversation, area);
}

pub(super) fn render_streaming_preview(content: &str) -> Text<'static> {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            " Streaming ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]));
    for line in content.lines() {
        lines.push(Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::LightCyan),
        )));
    }
    lines.push(Line::from(Span::styled(
        "...",
        Style::default().fg(Color::DarkGray),
    )));
    Text::from(lines)
}

pub(super) fn render_messages<'a>(messages: impl Iterator<Item = &'a UiMessage>) -> Text<'static> {
    let mut lines = Vec::new();
    for message in messages {
        let tone_style = match message.tone {
            UiTone::User => Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
            UiTone::Assistant => Style::default()
                .fg(Color::Black)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            UiTone::System => Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            UiTone::Error => Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", message.title), tone_style),
            Span::raw(" "),
        ]));
        for body_line in message.body.lines() {
            lines.push(Line::from(Span::raw(body_line.to_string())));
        }
        lines.push(Line::default());
    }
    Text::from(lines)
}

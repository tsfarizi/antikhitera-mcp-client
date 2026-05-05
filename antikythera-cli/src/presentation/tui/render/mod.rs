use antikythera_core::application::resilience::HealthStatus;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use super::app::ChatApp;
use super::types::{UiMessage, UiTone};

pub(crate) mod history_overlay;
pub(crate) mod log_panel;
pub(crate) mod settings_overlay;

pub(super) fn draw(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
    // ── Vertical skeleton ───────────────────────────────────────────────────
    //  [0] header (3 rows)
    //  [1] content area (min 16 rows)
    //  [2] prompt / model-edit bar (3 rows)
    //  [3] footer / status (2 rows)
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(16),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    // ── Horizontal split: chat | right panel ───────────────────────────────
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(layout[1]);

    // Right panel: context (top 40%) + WASM/FFI log (bottom 60%)
    let right_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(content[1]);

    // ── Header ──────────────────────────────────────────────────────────────
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " Antikythera CLI ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} / {}", app.provider, app.model),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            if app.agent_mode {
                "Agent Loop"
            } else {
                "Direct Chat"
            },
            Style::default().fg(Color::Yellow),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Session"));
    frame.render_widget(header, layout[0]);

    // ── Conversation ────────────────────────────────────────────────────────
    // Render all messages; the Paragraph wraps and scrolls to show the
    // viewport the user selected. When scroll = u16::MAX the viewport stays
    // pinned to the latest messages.
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
    // Append live streaming tokens as an in-progress assistant message.
    if app.loading && !app.streaming_content.is_empty() {
        conv_text.extend(render_streaming_preview(&app.streaming_content));
    }
    let conversation = Paragraph::new(conv_text)
        .block(Block::default().borders(Borders::ALL).title(conv_title))
        .wrap(Wrap { trim: false })
        .scroll((app.conversation_scroll, 0));
    frame.render_widget(conversation, content[0]);

    // ── Context sidebar ─────────────────────────────────────────────────────
    let sidebar_items = build_sidebar_items(app)
        .into_iter()
        .map(ListItem::new)
        .collect::<Vec<_>>();
    let sidebar =
        List::new(sidebar_items).block(Block::default().borders(Borders::ALL).title("Context"));
    frame.render_widget(sidebar, right_panel[0]);

    // ── WASM / FFI log panel ────────────────────────────────────────────────
    // Source prefixes (set by module loggers + SDK logging):
    //   cli:*     — CLI crate (streaming, HTTP clients)  → LightYellow
    //   sdk:*     — SDK / FFI layer (ConfigFfiLogger, WasmAgentLogger) → Magenta
    //   ffi:*     — WASM host runner events              → Magenta
    //   stream:*  — Model HTTP client send/recv          → Cyan
    //   agent:*   — Agent FSM, runner, context, parser   → Green
    //   tool:*    — MCP tool transports (SSE/RPC/proc)  → Blue
    //   provider  — Model provider API calls/replies     → Yellow
    //   transport — MCP transport (connect/disconnect)   → Blue
    //   config    — Configuration changes                → Gray
    //   session   — Session lifecycle                    → Gray
    //   core:*    — antikythera-core misc (client, svc)  → Gray
    // ERROR entries are shown in the chat area — still show in log.
    // Long lines wrap to the next row(s) instead of being truncated.
    let log_panel_area = right_panel[1];

    let all_log_lines: Vec<&String> = app.log_lines.iter().collect();
    // Build styled text lines — each log line becomes one or more wrapped
    // display lines via the Paragraph widget's Wrap behaviour.
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

    // ── Prompt bar ───────────────────────────────────────────────────────────
    {
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
        frame.render_widget(input_widget, layout[2]);
    }

    // ── Command autocomplete overlay ────────────────────────────────────────
    if app.input.starts_with('/') {
        let suggestions = app
            .suggestions()
            .into_iter()
            .map(|(name, description)| ListItem::new(format!("/{name:<10} {description}")))
            .collect::<Vec<_>>();
        let area = centered_rect(72, 34, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(
            List::new(suggestions).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Command Suggestions"),
            ),
            area,
        );
    }

    // ── Footer / status ──────────────────────────────────────────────────────
    // Health dot: read without blocking (try_lock avoids hanging the render loop).
    let (health_dot, health_color) = match app.health.try_lock() {
        Ok(h) => match h.overall_status() {
            HealthStatus::Healthy => ("\u{25cf} ", Color::Green),
            HealthStatus::Degraded => ("\u{25cf} ", Color::Yellow),
            HealthStatus::Unhealthy => ("\u{25cf} ", Color::Red),
        },
        Err(_) => ("\u{25cb} ", Color::Gray),
    };
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            health_dot,
            Style::default()
                .fg(health_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" autocomplete  "),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" submit  "),
        Span::styled(
            "F2",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" settings  "),
        Span::styled(
            "F3",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" riwayat  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" quit  "),
        Span::styled(
            "↑↓",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" scroll chat  "),
        Span::styled(
            "Ctrl+↑↓",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" scroll log  "),
        Span::styled(app.status.as_str(), Style::default().fg(Color::Gray)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(footer, layout[3]);

    // ── Settings overlay (drawn on top of everything else) ───────────────────
    if app.settings.open {
        settings_overlay::draw_settings_overlay(frame, app);
    }

    // ── History overlay (drawn on top of everything else) ──────────────────
    if app.history.open {
        history_overlay::draw_history_overlay(frame, app);
    }
}

fn render_streaming_preview(content: &str) -> Text<'static> {
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

fn render_messages<'a>(messages: impl Iterator<Item = &'a UiMessage>) -> Text<'static> {
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

fn build_sidebar_items(app: &ChatApp) -> Vec<String> {
    let session = app.session_id.as_deref().unwrap_or("belum ada");
    let provider_lines = app
        .providers
        .iter()
        .map(|provider| {
            let marker = if provider.id == app.provider {
                "*"
            } else {
                " "
            };
            let models = provider
                .models
                .iter()
                .map(|model| {
                    model
                        .display_name
                        .clone()
                        .unwrap_or_else(|| model.name.clone())
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{marker} {} [{}]\n  {}",
                provider.id, provider.provider_type, models
            )
        })
        .collect::<Vec<_>>();

    vec![
        format!("Provider aktif : {}", app.provider),
        format!("Model aktif    : {}", app.model),
        format!(
            "Mode           : {}",
            if app.agent_mode { "agent" } else { "chat" }
        ),
        format!("Tools aktif    : {}", app.tools),
        format!("Session        : {}", session),
        String::new(),
        "Providers".to_string(),
        provider_lines.join("\n"),
    ]
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

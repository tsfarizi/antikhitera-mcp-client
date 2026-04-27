use antikythera_core::application::resilience::HealthStatus;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::infrastructure::history::{ChatHistorySession, TurnRole};

use super::app::ChatApp;
use super::types::{PromptField, SettingsTab, UiMessage, UiTone};

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
    // Derive how many messages to show from the actual panel height so the
    // conversation area fills the terminal regardless of window size.
    // Each message occupies at least 2 rows (header + blank line), so dividing
    // by 2 gives a conservative upper bound; the Paragraph wraps the rest.
    let max_visible = ((content[0].height.saturating_sub(2)) / 2).max(4) as usize;
    let messages = app
        .messages
        .iter()
        .rev()
        .take(max_visible)
        .collect::<Vec<_>>();
    let conv_title = if app.loading {
        if app.streaming_content.is_empty() {
            "Conversation  [mengirim...]"
        } else {
            "Conversation  [menerima...]"
        }
    } else {
        "Conversation"
    };
    let mut conv_text = render_messages(messages.into_iter().rev());
    // Append live streaming tokens as an in-progress assistant message.
    if app.loading && !app.streaming_content.is_empty() {
        conv_text.extend(render_streaming_preview(&app.streaming_content));
    }
    let conversation = Paragraph::new(conv_text)
        .block(Block::default().borders(Borders::ALL).title(conv_title))
        .wrap(Wrap { trim: false });
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
    // Source prefixes (set by the tracing bridge + SDK logging):
    //   cli:*    — CLI crate (streaming, HTTP clients)  → LightYellow
    //   sdk:*    — SDK / FFI layer (ConfigFfiLogger, WasmAgentLogger) → Magenta
    //   ffi:*    — WASM host runner events              → Magenta
    //   stream:* — Model HTTP client send/recv          → Cyan
    //   agent:*  — Agent FSM, runner, context, parser   → Green
    //   tool:*   — MCP tool transports (SSE/RPC/proc)  → Blue
    //   core:*   — antikythera-core misc (client, svc)  → Gray
    // ERROR entries are already shown in the chat area — suppress here.
    // Only the most recent lines that fit the visible panel area are shown so
    // the latest activity is always visible without scrolling.
    let log_panel_height = right_panel[1].height.saturating_sub(2) as usize;
    let all_log_lines: Vec<&String> = app
        .log_lines
        .iter()
        .filter(|line| !line.contains("[ERROR]"))
        .collect();
    let log_start = all_log_lines.len().saturating_sub(log_panel_height);
    let log_items: Vec<ListItem<'_>> = all_log_lines[log_start..]
        .iter()
        .copied()
        .map(|line| {
            let style = if line.contains("[WARN]") {
                // Warnings always in yellow regardless of source.
                Style::default().fg(Color::Yellow)
            } else if line.contains("][cli:") {
                if line.contains("[DEBUG]") {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::LightYellow)
                }
            } else if line.contains("][sdk:") || line.contains("][ffi:") {
                if line.contains("[DEBUG]") {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default().fg(Color::LightMagenta)
                }
            } else if line.contains("][stream:") {
                if line.contains("[DEBUG]") {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::LightCyan)
                }
            } else if line.contains("][agent:") {
                if line.contains("[DEBUG]") {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::LightGreen)
                }
            } else if line.contains("][tool:") {
                if line.contains("[DEBUG]") {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default().fg(Color::LightBlue)
                }
            } else {
                // core:* or unknown
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Span::styled(line.clone(), style))
        })
        .collect();
    let log_panel = List::new(log_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Logs [yellow=CLI | magenta=FFI/SDK | cyan=stream | green=agent | blue=tool]")
            .title_style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    frame.render_widget(log_panel, right_panel[1]);

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
        Span::styled(app.status.as_str(), Style::default().fg(Color::Gray)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(footer, layout[3]);

    // ── Settings overlay (drawn on top of everything else) ───────────────────
    if app.settings.open {
        draw_settings_overlay(frame, app);
    }

    // ── History overlay (drawn on top of everything else) ──────────────────
    if app.history.open {
        draw_history_overlay(frame, app);
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

fn render_history_detail(session: &ChatHistorySession, scroll: usize) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    for turn in session.turns.iter().skip(scroll).take(20) {
        let ts = turn
            .timestamp
            .get(..19)
            .unwrap_or(turn.timestamp.as_str())
            .to_string();
        let (label, label_style) = match turn.role {
            TurnRole::User => (
                format!(" Anda [{ts}] "),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            TurnRole::Assistant => (
                format!(" AI   [{ts}] "),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        };
        lines.push(Line::from(Span::styled(label, label_style)));
        for body_line in turn.content.lines().take(10) {
            lines.push(Line::from(Span::raw(body_line.to_string())));
        }
        if turn.tool_steps > 0 {
            lines.push(Line::from(Span::styled(
                format!("  [{} langkah tool]", turn.tool_steps),
                Style::default().fg(Color::Yellow),
            )));
        }
        lines.push(Line::default());
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::raw("(sesi kosong)".to_string())));
    }
    Text::from(lines)
}

fn draw_history_overlay(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(" Riwayat Chat  [\u{2191}\u{2193} = navigasi  |  Enter = lihat  |  d = hapus  |  r = ganti judul  |  Esc = tutup] ")
        .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    if app.history.sessions.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "Belum ada riwayat sesi tersimpan.\n\
                 Riwayat disimpan otomatis setelah setiap respons AI.\n\
                 Tekan Esc untuk menutup.",
            )
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }

    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(inner);

    // ── Left: session list ─────────────────────────────────────────────────
    let list_items: Vec<ListItem> = app
        .history
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let cursor = i == app.history.cursor;
            let arrow = if cursor { "▶" } else { " " };
            let date = s.updated_at.get(..10).unwrap_or(s.updated_at.as_str());
            let title = if s.title.is_empty() {
                "<tanpa judul>"
            } else {
                s.title.as_str()
            };
            let style = if cursor {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!(
                "{arrow} {title}\n    {}/{} · {date} · {} giliran",
                s.provider,
                s.model,
                s.turns.len()
            ))
            .style(style)
        })
        .collect();

    frame.render_widget(
        List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "Sesi ({})  [Enter=lihat  d=hapus  r=ganti judul]",
                    app.history.sessions.len()
                ))
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        split[0],
    );

    // ── Right: detail or summary ───────────────────────────────────────────
    if let Some(detail) = &app.history.detail {
        let detail_title = if detail.title.is_empty() {
            "<tanpa judul>".to_string()
        } else {
            detail.title.clone()
        };
        frame.render_widget(
            Paragraph::new(render_history_detail(detail, app.history.detail_scroll))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(
                            "Percakapan: {}  [\u{2191}\u{2193}=gulir  Esc=kembali]",
                            detail_title
                        ))
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .wrap(Wrap { trim: false }),
            split[1],
        );
    } else if let Some(s) = app.history.sessions.get(app.history.cursor) {
        let summary = format!(
            "Provider  : {}\nModel     : {}\nMode      : {}\nGiliran   : {}\nDibuat    : {}\nDiperbarui: {}\nCore ID   : {}\n\nEnter = lihat percakapan\nd     = hapus sesi\nr     = ganti judul",
            s.provider,
            s.model,
            if s.agent_mode {
                "Agent Loop"
            } else {
                "Chat Langsung"
            },
            s.turns.len(),
            s.created_at.get(..19).unwrap_or(s.created_at.as_str()),
            s.updated_at.get(..19).unwrap_or(s.updated_at.as_str()),
            s.core_session_id.as_deref().unwrap_or("-"),
        );
        frame.render_widget(
            Paragraph::new(summary)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Info Sesi")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .wrap(Wrap { trim: false }),
            split[1],
        );
    }

    // ── Rename input bar overlay at the bottom ─────────────────────────────
    if app.history.rename_mode {
        let bottom = Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(4),
            width: area.width.saturating_sub(2),
            height: 3,
        };
        frame.render_widget(Clear, bottom);
        frame.render_widget(
            Paragraph::new(format!("Judul baru: {}\u{2588}", app.history.rename_buffer))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Ganti Judul  [Enter=simpan  Esc=batal]")
                        .border_style(Style::default().fg(Color::Magenta)),
                )
                .style(Style::default().fg(Color::Magenta)),
            bottom,
        );
    }
}

// ── Settings overlay draw functions ──────────────────────────────────────────

fn draw_settings_overlay(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
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

fn draw_settings_tab_provider(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
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

fn draw_settings_tab_model(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
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

fn draw_settings_tab_prompts(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
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

fn draw_settings_tab_system(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
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

fn draw_settings_tab_agent(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
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

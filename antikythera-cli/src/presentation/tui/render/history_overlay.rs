//! History browser overlay rendering.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::infrastructure::history::{ChatHistorySession, TurnRole};

use super::super::app::ChatApp;

pub(super) fn render_history_detail(session: &ChatHistorySession, scroll: usize) -> Text<'static> {
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

pub(super) fn draw_history_overlay(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
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

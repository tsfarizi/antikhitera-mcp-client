//! Settings overlay with Provider, Model, Prompts, System, Agent tabs.

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::super::app::ChatApp;
use super::super::types::SettingsTab;

pub(super) fn draw_settings_overlay(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(" \u{2699}  Settings  [Tab/BackTab=ganti tab | \u{2191}\u{2193}=nav | Enter=pilih | Ctrl+S=simpan | Esc=tutup] ")
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

    match app.settings.tab {
        SettingsTab::Provider => {
            super::provider_tab::draw_settings_tab_provider(frame, app, layout[1])
        }
        SettingsTab::Model => super::model_tab::draw_settings_tab_model(frame, app, layout[1]),
        SettingsTab::Prompts => {
            super::prompts_tab::draw_settings_tab_prompts(frame, app, layout[1])
        }
        SettingsTab::System => super::system_tab::draw_settings_tab_system(frame, app, layout[1]),
        SettingsTab::Agent => super::agent_tab::draw_settings_tab_agent(frame, app, layout[1]),
    }
}

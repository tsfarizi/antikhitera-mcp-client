//! Context sidebar showing active provider, model, mode, tools, session, and providers list.

use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, List, ListItem};

use super::super::app::ChatApp;

pub(super) fn draw_sidebar(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let sidebar_items = build_sidebar_items(app)
        .into_iter()
        .map(ListItem::new)
        .collect::<Vec<_>>();
    let sidebar =
        List::new(sidebar_items).block(Block::default().borders(Borders::ALL).title("Context"));
    frame.render_widget(sidebar, area);
}

pub(super) fn build_sidebar_items(app: &ChatApp) -> Vec<String> {
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

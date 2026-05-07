//! Command autocomplete overlay shown when typing `/` commands.

use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use super::super::app::ChatApp;

pub(super) fn draw_autocomplete(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    if !app.input.starts_with('/') {
        return;
    }
    let suggestions = app
        .suggestions()
        .into_iter()
        .map(|(name, description)| ListItem::new(format!("/{name:<10} {description}")))
        .collect::<Vec<_>>();
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

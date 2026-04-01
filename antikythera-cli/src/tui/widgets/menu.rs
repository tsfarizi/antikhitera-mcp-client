//! Menu widget for TUI navigation

use super::super::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// Menu item with optional marker for default/selected
pub struct MenuItem {
    pub label: String,
    pub is_default: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            is_default: false,
        }
    }

    pub fn with_default_marker(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }
}

/// Menu widget state
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub state: ListState,
    pub title: String,
    pub subtitle: Option<String>,
}

impl Menu {
    pub fn new(title: impl Into<String>, items: Vec<MenuItem>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            state,
            title: title.into(),
            subtitle: None,
        }
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn select(&mut self, index: usize) {
        if index < self.items.len() {
            self.state.select(Some(index));
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3), // Header
            Constraint::Length(2), // Subtitle
            Constraint::Min(0),    // List
            Constraint::Length(2), // Footer
        ])
        .split(area);

        // Header with theme styling
        let header = Paragraph::new(self.title.clone())
            .style(theme::title())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(theme::border()),
            );
        frame.render_widget(header, chunks[0]);

        // Subtitle with muted style
        if let Some(subtitle) = &self.subtitle {
            let sub = Paragraph::new(subtitle.clone())
                .style(theme::subtitle())
                .alignment(Alignment::Center);
            frame.render_widget(sub, chunks[1]);
        }

        // List items with refined styling
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                let marker = if item.is_default { " ★" } else { "" };
                let content = Line::from(vec![
                    Span::raw(&item.label),
                    Span::styled(marker, theme::default_marker()),
                ]);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .highlight_style(theme::selected())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, chunks[2], &mut self.state);

        // Footer with muted style
        let footer = Paragraph::new("↑↓ Navigate  Enter Select  Esc Back  q Quit")
            .style(theme::footer())
            .alignment(Alignment::Center);
        frame.render_widget(footer, chunks[3]);
    }
}

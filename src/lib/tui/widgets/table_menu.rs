//! Table-based menu widget for provider/model display

use super::super::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::Span,
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
};

/// Table row for provider/model display
pub struct TableRow {
    pub cells: Vec<String>,
    pub is_default: bool,
}

impl TableRow {
    pub fn new(cells: Vec<String>) -> Self {
        Self {
            cells,
            is_default: false,
        }
    }

    pub fn with_default_marker(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }
}

/// Table-based menu widget
pub struct TableMenu {
    pub title: String,
    pub subtitle: Option<String>,
    pub headers: Vec<String>,
    pub rows: Vec<TableRow>,
    pub selected: usize,
    pub actions: Vec<String>,
}

impl TableMenu {
    pub fn new(
        title: impl Into<String>,
        headers: Vec<String>,
        rows: Vec<TableRow>,
        actions: Vec<String>,
    ) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            headers,
            rows,
            selected: 0,
            actions,
        }
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn total_items(&self) -> usize {
        self.rows.len() + self.actions.len()
    }

    pub fn next(&mut self) {
        let total = self.total_items();
        if total > 0 {
            self.selected = (self.selected + 1) % total;
        }
    }

    pub fn previous(&mut self) {
        let total = self.total_items();
        if total > 0 {
            self.selected = if self.selected == 0 {
                total - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn is_row_selected(&self) -> bool {
        self.selected < self.rows.len()
    }

    pub fn selected_action_index(&self) -> Option<usize> {
        if self.selected >= self.rows.len() {
            Some(self.selected - self.rows.len())
        } else {
            None
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3), // Header
            Constraint::Length(2), // Subtitle
            Constraint::Min(0),    // Table + Actions
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

        // Subtitle
        if let Some(subtitle) = &self.subtitle {
            let sub = Paragraph::new(subtitle.clone())
                .style(theme::subtitle())
                .alignment(Alignment::Center);
            frame.render_widget(sub, chunks[1]);
        }

        let content_chunks = Layout::vertical([
            Constraint::Min(0),                                  // Table
            Constraint::Length((self.actions.len() + 1) as u16), // Actions
        ])
        .split(chunks[2]);

        // Table header row
        let header_row = Row::new(
            self.headers
                .iter()
                .map(|h| Span::styled(h.clone(), theme::title())),
        );

        // Table data rows
        let table_rows: Vec<Row> = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let marker = if row.is_default { " ★" } else { "" };
                let mut cells: Vec<Span> = row.cells.iter().map(|c| Span::raw(c.clone())).collect();
                if !cells.is_empty() {
                    let last_idx = cells.len() - 1;
                    let last_text = format!("{}{}", cells[last_idx].content, marker);
                    cells[last_idx] = if row.is_default {
                        Span::styled(last_text, theme::default_marker())
                    } else {
                        Span::raw(last_text)
                    };
                }
                let style = if i == self.selected {
                    theme::selected()
                } else {
                    Style::default()
                };
                Row::new(cells).style(style)
            })
            .collect();

        let widths: Vec<Constraint> = self
            .headers
            .iter()
            .map(|_| Constraint::Percentage(100 / self.headers.len() as u16))
            .collect();

        let table = Table::new(table_rows, widths)
            .header(header_row.style(theme::title()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme::border()),
            );
        frame.render_widget(table, content_chunks[0]);

        // Action items
        let action_items: Vec<ListItem> = self
            .actions
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let style = if self.selected == self.rows.len() + i {
                    theme::selected()
                } else {
                    theme::action()
                };
                ListItem::new(format!("  {}", action)).style(style)
            })
            .collect();

        let actions_list = List::new(action_items);
        frame.render_widget(actions_list, content_chunks[1]);

        // Footer
        let footer = Paragraph::new("↑↓ Navigate  Enter Select  Esc Back  q Quit")
            .style(theme::footer())
            .alignment(Alignment::Center);
        frame.render_widget(footer, chunks[3]);
    }
}

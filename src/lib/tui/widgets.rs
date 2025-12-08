//! Reusable TUI widgets

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// Menu widget state
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub state: ListState,
    pub title: String,
    pub subtitle: Option<String>,
}

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
        let header = Paragraph::new(self.title.clone())
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);
        if let Some(subtitle) = &self.subtitle {
            let sub = Paragraph::new(subtitle.clone())
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(sub, chunks[1]);
        }
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                let marker = if item.is_default { " ★" } else { "" };
                let content = Line::from(vec![
                    Span::raw(&item.label),
                    Span::styled(marker, Style::default().fg(Color::Yellow)),
                ]);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, chunks[2], &mut self.state);
        let footer = Paragraph::new("↑↓ Navigate  Enter Select  Esc Back  q Quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(footer, chunks[3]);
    }
}

/// Text input widget for TUI forms
pub struct TextInput {
    pub value: String,
    pub label: String,
    pub cursor_pos: usize,
}

impl TextInput {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            value: String::new(),
            label: label.into(),
            cursor_pos: 0,
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self.cursor_pos = self.value.len();
        self
    }

    pub fn handle_char(&mut self, c: char) {
        self.value.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }

    pub fn handle_backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.value.remove(self.cursor_pos);
        }
    }

    pub fn handle_delete(&mut self) {
        if self.cursor_pos < self.value.len() {
            self.value.remove(self.cursor_pos);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_pos < self.value.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(1), // Label
            Constraint::Length(3), // Input box
        ])
        .split(area);

        let label = Paragraph::new(self.label.clone()).style(Style::default().fg(Color::Yellow));
        frame.render_widget(label, chunks[0]);

        let input = Paragraph::new(self.value.clone())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            );
        frame.render_widget(input, chunks[1]);
    }
}

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
        use ratatui::widgets::{Row, Table};

        let chunks = Layout::vertical([
            Constraint::Length(3), // Header
            Constraint::Length(2), // Subtitle
            Constraint::Min(0),    // Table + Actions
            Constraint::Length(2), // Footer
        ])
        .split(area);
        let header = Paragraph::new(self.title.clone())
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);
        if let Some(subtitle) = &self.subtitle {
            let sub = Paragraph::new(subtitle.clone())
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(sub, chunks[1]);
        }
        let content_chunks = Layout::vertical([
            Constraint::Min(0),                                  // Table
            Constraint::Length((self.actions.len() + 1) as u16), // Actions
        ])
        .split(chunks[2]);
        let header_row = Row::new(
            self.headers
                .iter()
                .map(|h| Span::styled(h.clone(), Style::default().add_modifier(Modifier::BOLD))),
        );

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
                        Span::styled(last_text, Style::default().fg(Color::Yellow))
                    } else {
                        Span::raw(last_text)
                    };
                }
                let style = if i == self.selected {
                    Style::default().bg(Color::Blue).fg(Color::White)
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
            .header(header_row.style(Style::default().fg(Color::Cyan)))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(table, content_chunks[0]);
        let action_items: Vec<ListItem> = self
            .actions
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let style = if self.selected == self.rows.len() + i {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };
                ListItem::new(format!("  {}", action)).style(style)
            })
            .collect();

        let actions_list = List::new(action_items);
        frame.render_widget(actions_list, content_chunks[1]);
        let footer = Paragraph::new("↑↓ Navigate  Enter Select  Esc Back  q Quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(footer, chunks[3]);
    }
}

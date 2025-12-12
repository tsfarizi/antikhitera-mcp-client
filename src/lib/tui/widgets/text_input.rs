//! Text input widget for TUI forms

use super::super::theme;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};

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

        let label = Paragraph::new(self.label.clone()).style(theme::default_marker());
        frame.render_widget(label, chunks[0]);

        let input = Paragraph::new(self.value.clone())
            .style(theme::text())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme::border_active()),
            );
        frame.render_widget(input, chunks[1]);
    }
}

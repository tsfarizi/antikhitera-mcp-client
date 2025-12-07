//! Chat state management

use crate::agent::AgentStep;

/// A single chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            timestamp: std::time::Instant::now(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: std::time::Instant::now(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            timestamp: std::time::Instant::now(),
        }
    }
}

/// Chat session state
pub struct ChatState {
    /// Message history
    pub messages: Vec<ChatMessage>,
    /// Current input buffer
    pub input: String,
    /// Cursor position in input
    pub cursor_pos: usize,
    /// Scroll offset for messages
    pub scroll_offset: u16,
    /// Whether agent mode is enabled
    pub agent_mode: bool,
    /// Current session ID
    pub session_id: Option<String>,
    /// Whether waiting for response
    pub loading: bool,
    /// Loading animation frame
    pub loading_frame: usize,
    /// Last logs from interaction
    pub last_logs: Vec<String>,
    /// Last tool steps from agent
    pub last_steps: Vec<AgentStep>,
    /// Status message
    pub status_message: Option<String>,
}

impl Default for ChatState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            agent_mode: true,
            session_id: None,
            loading: false,
            loading_frame: 0,
            last_logs: Vec::new(),
            last_steps: Vec::new(),
            status_message: None,
        }
    }

    /// Add a message to history
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        // Auto-scroll to bottom
        self.scroll_to_bottom();
    }

    /// Get the current input and clear it
    pub fn take_input(&mut self) -> String {
        self.cursor_pos = 0;
        std::mem::take(&mut self.input)
    }

    /// Insert character at cursor position
    pub fn insert_char(&mut self, c: char) {
        if self.cursor_pos >= self.input.len() {
            self.input.push(c);
        } else {
            self.input.insert(self.cursor_pos, c);
        }
        self.cursor_pos += 1;
    }

    /// Delete character before cursor (backspace)
    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 && !self.input.is_empty() {
            self.input.remove(self.cursor_pos - 1);
            self.cursor_pos -= 1;
        }
    }

    /// Delete character at cursor (delete key)
    pub fn delete_char_forward(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        self.cursor_pos = self.cursor_pos.saturating_sub(1);
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }

    /// Move cursor to start
    pub fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    /// Move cursor to end
    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.input.len();
    }

    /// Scroll messages up
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll messages down
    pub fn scroll_down(&mut self, max_scroll: u16) {
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Scroll to bottom of messages
    pub fn scroll_to_bottom(&mut self) {
        // Will be calculated during render based on content height
        self.scroll_offset = u16::MAX;
    }

    /// Reset session
    pub fn reset(&mut self) {
        self.messages.clear();
        self.session_id = None;
        self.last_logs.clear();
        self.last_steps.clear();
        self.scroll_offset = 0;
        self.status_message = Some("Session reset".into());
    }

    /// Toggle agent mode
    pub fn toggle_agent_mode(&mut self) {
        self.agent_mode = !self.agent_mode;
        self.status_message = Some(format!(
            "Agent mode: {}",
            if self.agent_mode { "ON" } else { "OFF" }
        ));
    }

    /// Update loading animation frame
    pub fn tick_loading(&mut self) {
        if self.loading {
            self.loading_frame = (self.loading_frame + 1) % 4;
        }
    }

    /// Check if input is a command
    pub fn is_command(&self) -> bool {
        self.input.starts_with('/') || self.input.starts_with(':')
    }

    /// Get command name if input is a command
    pub fn get_command(&self) -> Option<&str> {
        if self.is_command() {
            let cmd = self.input.trim_start_matches(|c| c == '/' || c == ':');
            cmd.split_whitespace().next()
        } else {
            None
        }
    }
}

//! TUI Unit Tests
//!
//! Comprehensive tests for all Ratatui TUI components:
//! - Widgets: Menu, MenuItem, TableMenu
//! - Chat: ChatState, ChatMessage, input handling
//! - Terminal: NavAction key mapping

mod tui;

// Re-export tests
pub use tui::*;

//! TUI Chat interface module
//!
//! This module provides a Ratatui-based interactive chat interface
//! following SOLID principles:
//! - state.rs: Single responsibility for chat state management
//! - ui.rs: Single responsibility for UI rendering
//! - input.rs: Single responsibility for input handling
//! - runner.rs: Coordinates the components

mod input;
mod runner;
mod state;
mod ui;

// Re-exports
pub use runner::{ChatResult, run_chat};
pub use state::{ChatMessage, ChatState, MessageRole};

//! Reusable TUI widgets
//!
//! This module provides common widgets:
//! - [`Menu`] - Simple list menu with navigation
//! - [`MenuItem`] - Menu item with optional default marker
//! - [`TextInput`] - Text input field with cursor
//! - [`TableMenu`] - Table-based menu for data display
//! - [`TableRow`] - Row data for TableMenu

mod menu;
mod table_menu;
mod text_input;

pub use menu::{Menu, MenuItem};
pub use table_menu::{TableMenu, TableRow};
pub use text_input::TextInput;

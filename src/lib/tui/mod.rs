//! TUI module for terminal user interface using Ratatui
//!
//! Provides full-screen interactive menus with arrow key navigation.

pub mod screens;
mod terminal;
mod widgets;

pub use terminal::{Tui, restore_terminal};
pub use widgets::Menu;
pub use widgets::{TableMenu, TableRow, TextInput};

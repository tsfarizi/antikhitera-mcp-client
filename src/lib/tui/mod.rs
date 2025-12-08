//! TUI module for terminal user interface using Ratatui
//!
//! Provides full-screen interactive menus with arrow key navigation.

pub mod screens;
pub mod terminal;
pub mod theme;
pub mod widgets;

pub use terminal::{NavAction, Tui, restore_terminal};
pub use widgets::{Menu, MenuItem};
pub use widgets::{TableMenu, TableRow, TextInput};

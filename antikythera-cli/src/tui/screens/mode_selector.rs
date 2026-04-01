//! Mode selector screen

use crate::cli::RunMode;
use crate::tui::terminal::{NavAction, init_terminal, read_key, restore_terminal};
use crate::tui::widgets::{Menu, MenuItem};
use std::io;

/// Version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// GitHub repository URL
const GITHUB_URL: &str = "https://github.com/tsfarizi/antikhitera-mcp-client";

/// Run the mode selector TUI and return selected mode
/// Press 'q' to quit the program entirely
pub fn run_mode_selector() -> io::Result<Option<RunMode>> {
    let mut terminal = init_terminal()?;

    let items = vec![
        MenuItem::new("CLI   - Debug & Native mode"),
        MenuItem::new("WASM  - WebAssembly build target"),
    ];

    let title = format!("🚀 Antikythera MCP v{}", VERSION);
    let subtitle = format!("📦 {}  |  ↑↓ Navigate  Enter Select  q Quit", GITHUB_URL);
    let mut menu = Menu::new(title, items).with_subtitle(subtitle);

    let result = loop {
        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => menu.previous(),
            NavAction::Down => menu.next(),
            NavAction::Select => {
                let mode = match menu.selected_index() {
                    Some(0) => Some(RunMode::Cli),
                    Some(1) => Some(RunMode::Wasm),
                    _ => None,
                };
                break mode;
            }
            NavAction::ForceQuit => {
                // 'q' pressed - exit program entirely
                restore_terminal()?;
                return Ok(None);
            }
            NavAction::Back => break None,
            NavAction::None => {}
        }
    };

    restore_terminal()?;
    Ok(result)
}

//! Mode selector screen

use crate::cli::RunMode;
use crate::tui::terminal::{NavAction, init_terminal, read_key, restore_terminal};
use crate::tui::widgets::{Menu, MenuItem};
use std::io;

/// Run the mode selector TUI and return selected mode
pub fn run_mode_selector() -> io::Result<Option<RunMode>> {
    let mut terminal = init_terminal()?;

    let items = vec![
        MenuItem::new("STDIO - Interactive chat"),
        MenuItem::new("REST  - API server"),
        MenuItem::new("Both  - Run STDIO + REST"),
        MenuItem::new("Setup - Configuration wizard"),
    ];

    let mut menu = Menu::new("🚀 MCP Client - Mode Selection", items);

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
                    Some(0) => Some(RunMode::Stdio),
                    Some(1) => Some(RunMode::Rest),
                    Some(2) => Some(RunMode::All),
                    Some(3) => Some(RunMode::Setup),
                    _ => None,
                };
                break mode;
            }
            NavAction::ForceQuit | NavAction::Back => break None,
            NavAction::None => {}
        }
    };

    restore_terminal()?;
    Ok(result)
}

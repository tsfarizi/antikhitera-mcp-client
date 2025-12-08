//! Setup menu screen with TUI - Main entry point

mod models;
mod prompt;
mod providers;
mod servers;
mod sync;

use crate::config::{AppConfig, CONFIG_PATH};
use crate::tui::terminal::{NavAction, Tui, init_terminal, read_key, restore_terminal};
use crate::tui::widgets::{Menu, MenuItem};
use std::error::Error;
use std::path::Path;

/// Setup menu result
pub enum SetupResult {
    Back,
    Exit,
}

/// Run the setup menu TUI
pub fn run_setup_menu() -> Result<SetupResult, Box<dyn Error>> {
    let mut terminal = init_terminal()?;

    let items = vec![
        MenuItem::new("Manage Providers"),
        MenuItem::new("Manage Models"),
        MenuItem::new("Manage MCP Servers"),
        MenuItem::new("Manage Prompt Template"),
        MenuItem::new("← Back"),
    ];

    let mut menu = Menu::new("🔧 Setup Menu", items);

    let result = loop {
        terminal.draw(|frame| {
            menu.render(frame, frame.area());
        })?;

        let action = NavAction::from(read_key()?);
        match action {
            NavAction::Up => menu.previous(),
            NavAction::Down => menu.next(),
            NavAction::Select => match menu.selected_index() {
                Some(0) => {
                    run_manage_providers_tui(&mut terminal)?;
                }
                Some(1) => {
                    run_manage_models_tui(&mut terminal)?;
                }
                Some(2) => {
                    run_manage_servers_tui(&mut terminal)?;
                }
                Some(3) => {
                    run_edit_prompt_tui(&mut terminal)?;
                }
                Some(4) => break SetupResult::Back,
                _ => {}
            },
            NavAction::ForceQuit => break SetupResult::Exit,
            NavAction::Back => break SetupResult::Back,
            NavAction::None => {}
        }
    };

    restore_terminal()?;
    Ok(result)
}

/// Load configuration from default path
pub(crate) fn load_config() -> Result<AppConfig, Box<dyn Error>> {
    AppConfig::load(Some(Path::new(CONFIG_PATH))).map_err(|e| Box::new(e) as Box<dyn Error>)
}

fn run_manage_providers_tui(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    providers::run_manage_providers_with_terminal(terminal)
}

fn run_manage_models_tui(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    models::run_manage_models_with_terminal(terminal)
}

fn run_manage_servers_tui(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    servers::run_manage_servers_with_terminal(terminal)
}

fn run_edit_prompt_tui(terminal: &mut Tui) -> Result<(), Box<dyn Error>> {
    prompt::run_edit_prompt_with_terminal(terminal)
}

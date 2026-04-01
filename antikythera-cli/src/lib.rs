//! # Antikythera CLI
//!
//! Terminal User Interface (TUI), command-line interface, and setup wizard.

pub mod cli;
pub mod tui;
pub mod wizard;

// Re-export commonly used types
pub use cli::{Cli, RunMode};

/// Run the CLI with the given arguments
pub async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    tui::run_tui_with_cli(cli).await
}

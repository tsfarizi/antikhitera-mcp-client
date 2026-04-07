//! Configuration wizard module for interactive setup
//!
//! ⚠️ DEPRECATED: Use antikythera-core::config::wizard instead.
//! This module is kept for backwards compatibility only.

pub mod generators;
pub mod prompts;
pub mod ui;
pub mod generator;

use antikythera_core::config::wizard as core_wizard;
use std::error::Error;

/// Run the initial setup wizard when no config exists
/// Delegates to core wizard
pub async fn run_wizard() -> Result<(), Box<dyn Error>> {
    core_wizard::run_wizard().await
}

/// Run the setup menu (accessible from mode selector)
/// Delegates to core wizard
pub async fn run_setup_menu() -> Result<bool, Box<dyn Error>> {
    core_wizard::run_setup_menu().await
}

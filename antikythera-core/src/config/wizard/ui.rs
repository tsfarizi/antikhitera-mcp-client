//! UI helpers for attractive CLI output

use std::io::{self, Write};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";
const DIM: &str = "\x1b[2m";

/// Print a header with box drawing
pub fn print_header(title: &str) {
    let width = 54;

    antikythera_log::cli_print!();
    antikythera_log::cli_print!("{}╔{}╗{}", CYAN, "═".repeat(width), RESET);
    antikythera_log::cli_print!(
        "{}║{:^width$}{}║{}",
        CYAN,
        format!("{}🔧  {}{}", BOLD, title, RESET),
        CYAN,
        RESET,
        width = width
    );
    antikythera_log::cli_print!("{}╚{}╝{}", CYAN, "═".repeat(width), RESET);
    antikythera_log::cli_print!();
}

/// Print a section divider with title
pub fn print_section(title: &str) {
    antikythera_log::cli_print!();
    antikythera_log::cli_print!("{}╠{}╣{}", CYAN, "═".repeat(54), RESET);
    antikythera_log::cli_print!("{}║  {}{}{}", CYAN, BOLD, title, RESET);
    antikythera_log::cli_print!("{}╠{}╣{}", CYAN, "═".repeat(54), RESET);
    antikythera_log::cli_print!();
}

/// Print a simple divider line
pub fn print_divider() {
    antikythera_log::cli_print!(
        "{}─────────────────────────────────────────────{}",
        DIM,
        RESET
    );
}

/// Print informational text
pub fn print_info(message: &str) {
    antikythera_log::cli_print!("  {}", message);
}

/// Print a hint (dimmed)
pub fn print_hint(message: &str) {
    antikythera_log::cli_print!("  {}→ {}{}", DIM, message, RESET);
}

/// Print success message
pub fn print_success(message: &str) {
    antikythera_log::cli_print!();
    antikythera_log::cli_print!("  {}{}✅ {}{}", GREEN, BOLD, message, RESET);
}

/// Print warning message
pub fn print_warning(message: &str) {
    antikythera_log::cli_print!("  {}⚠️  {}{}", YELLOW, message, RESET);
}

/// Print error message
pub fn print_error(message: &str) {
    antikythera_log::cli_print!("  {}❌ {}{}", RED, message, RESET);
}

/// Flush stdout
pub fn flush() {
    io::stdout().flush().ok();
}

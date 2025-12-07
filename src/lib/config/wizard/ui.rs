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

    println!();
    println!("{}╔{}╗{}", CYAN, "═".repeat(width), RESET);
    println!(
        "{}║{:^width$}{}║{}",
        CYAN,
        format!("{}🔧  {}{}", BOLD, title, RESET),
        CYAN,
        RESET,
        width = width
    );
    println!("{}╚{}╝{}", CYAN, "═".repeat(width), RESET);
    println!();
}

/// Print a section divider with title
pub fn print_section(title: &str) {
    println!();
    println!("{}╠{}╣{}", CYAN, "═".repeat(54), RESET);
    println!("{}║  {}{}{}", CYAN, BOLD, title, RESET);
    println!("{}╠{}╣{}", CYAN, "═".repeat(54), RESET);
    println!();
}

/// Print a simple divider line
pub fn print_divider() {
    println!(
        "{}─────────────────────────────────────────────{}",
        DIM, RESET
    );
}

/// Print informational text
pub fn print_info(message: &str) {
    println!("  {}", message);
}

/// Print a hint (dimmed)
pub fn print_hint(message: &str) {
    println!("  {}→ {}{}", DIM, message, RESET);
}

/// Print success message
pub fn print_success(message: &str) {
    println!();
    println!("  {}{}✅ {}{}", GREEN, BOLD, message, RESET);
}

/// Print warning message
pub fn print_warning(message: &str) {
    println!("  {}⚠️  {}{}", YELLOW, message, RESET);
}

/// Print error message
pub fn print_error(message: &str) {
    println!("  {}❌ {}{}", RED, message, RESET);
}

/// Flush stdout
pub fn flush() {
    io::stdout().flush().ok();
}

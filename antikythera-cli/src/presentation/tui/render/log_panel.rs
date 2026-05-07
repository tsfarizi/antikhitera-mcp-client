//! Log panel color resolution by source and level.

use ratatui::style::{Color, Style};

pub fn resolve_log_line_style(line: &str) -> Style {
    if line.contains("[WARN]") {
        return Style::default().fg(Color::Yellow);
    }
    if line.contains("[ERROR]") {
        return Style::default().fg(Color::Red);
    }
    let is_debug = line.contains("[DEBUG]");
    // SDK/FFI entries have a colon-prefixed source (e.g. "sdk:ConfigFfiLogger")
    if line.contains("][sdk:") || line.contains("][ffi:") {
        return if is_debug {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default().fg(Color::LightMagenta)
        };
    }
    if line.contains("][cli:") {
        return if is_debug {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::LightYellow)
        };
    }
    if line.contains("][stream:") {
        return if is_debug {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::LightCyan)
        };
    }
    // Module loggers from core — bare source names (no colon).
    if line.contains("][agent]") {
        return if is_debug {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::LightGreen)
        };
    }
    if line.contains("][provider]") {
        return if is_debug {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::LightYellow)
        };
    }
    if line.contains("][transport]") || line.contains("][tool]") {
        return if is_debug {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::LightBlue)
        };
    }
    if line.contains("][config]")
        || line.contains("][session]")
        || line.contains("][chat]")
        || line.contains("][discovery]")
        || line.contains("][resilience]")
        || line.contains("][streaming]")
        || line.contains("][orchestrator]")
        || line.contains("][stdio]")
        || line.contains("][wasm]")
        || line.contains("][security]")
    {
        return Style::default().fg(Color::Gray);
    }
    // core:* or unknown
    Style::default().fg(Color::Gray)
}

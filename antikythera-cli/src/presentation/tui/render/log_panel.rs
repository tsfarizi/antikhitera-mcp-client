//! Log panel color resolution by source and level.

use ratatui::style::{Color, Style};

pub(super) fn resolve_log_line_style(line: &str) -> Style {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Level-based styling ──────────────────────────────────────────────
    #[test]
    fn warn_level_yellow() {
        let line = "12:34:56 [WARN][agent] something happened";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Yellow));
    }

    #[test]
    fn error_level_red() {
        let line = "12:34:56 [ERROR][provider] connection failed";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Red));
    }

    #[test]
    fn debug_sdk_magenta() {
        let line = "12:34:56 [DEBUG][sdk:ffi] function called";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Magenta));
    }

    // ── Source-tag styling ───────────────────────────────────────────────
    #[test]
    fn agent_source_light_green() {
        let line = "12:34:56 [INFO][agent] agent started";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::LightGreen));
    }

    #[test]
    fn agent_source_debug_green() {
        let line = "12:34:56 [DEBUG][agent] tool step";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn provider_source_light_yellow() {
        let line = "12:34:56 [INFO][provider] api response";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::LightYellow));
    }

    #[test]
    fn transport_source_light_blue() {
        let line = "12:34:56 [INFO][transport] data sent";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::LightBlue));
    }

    #[test]
    fn tool_source_light_blue() {
        let line = "12:34:56 [INFO][tool] tool executed";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::LightBlue));
    }

    #[test]
    fn orchestrator_source_gray() {
        let line = "12:34:56 [INFO][orchestrator] routing request";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Gray));
    }

    #[test]
    fn security_source_gray() {
        let line = "12:34:56 [INFO][security] auth check";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Gray));
    }

    #[test]
    fn sdk_source_non_debug_light_magenta() {
        let line = "12:34:56 [INFO][sdk:ConfigFfiLogger] config updated";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::LightMagenta));
    }

    #[test]
    fn cli_source_light_yellow() {
        let line = "12:34:56 [INFO][cli:main] starting up";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::LightYellow));
    }

    // ── Unknown / default ────────────────────────────────────────────────
    #[test]
    fn unknown_source_defaults_to_gray() {
        let line = "12:34:56 [INFO][unknown_tag] some message";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Gray));
    }

    #[test]
    fn empty_source_defaults_to_gray() {
        let line = "12:34:56 [INFO][] message";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Gray));
    }

    #[test]
    fn warn_takes_priority_over_source_color() {
        let line = "12:34:56 [WARN][agent] warning from agent";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Yellow));
    }

    #[test]
    fn error_takes_priority_over_source_color() {
        let line = "12:34:56 [ERROR][sdk:ffi] sdk crash";
        let style = resolve_log_line_style(line);
        assert_eq!(style.fg, Some(Color::Red));
    }
}

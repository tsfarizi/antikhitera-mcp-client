use antikythera_cli::presentation::tui::render::log_panel::resolve_log_line_style;
use ratatui::style::Color;

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

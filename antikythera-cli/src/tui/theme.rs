//! TUI Theme - Consistent, elegant styling
//!
//! Color palette inspired by modern terminal aesthetics.
//! Uses subtle gradients of blue/cyan for primary actions,
//! with muted accents to avoid visual noise.

use ratatui::style::{Color, Modifier, Style};

/// Primary accent color - soft cyan blue
pub const ACCENT: Color = Color::Rgb(100, 180, 220);

/// Secondary accent - warm amber for highlights
pub const HIGHLIGHT: Color = Color::Rgb(255, 200, 100);

/// Success indicator - soft green
pub const SUCCESS: Color = Color::Rgb(130, 200, 130);

/// Error indicator - soft red
pub const ERROR: Color = Color::Rgb(220, 100, 100);

/// Muted text - for secondary information
pub const MUTED: Color = Color::Rgb(100, 100, 110);

/// Border color - subtle gray
pub const BORDER: Color = Color::Rgb(70, 75, 85);

/// Selected item background
pub const SELECTED_BG: Color = Color::Rgb(50, 60, 80);

/// Header/title style
pub fn title() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Subtitle/secondary text style
pub fn subtitle() -> Style {
    Style::default().fg(MUTED)
}

/// Normal text style
pub fn text() -> Style {
    Style::default().fg(Color::White)
}

/// Highlighted/selected item style
pub fn selected() -> Style {
    Style::default()
        .bg(SELECTED_BG)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Default marker (star) style
pub fn default_marker() -> Style {
    Style::default().fg(HIGHLIGHT)
}

/// Action/interactive element style
pub fn action() -> Style {
    Style::default().fg(SUCCESS)
}

/// Border style
pub fn border() -> Style {
    Style::default().fg(BORDER)
}

/// Active border style
pub fn border_active() -> Style {
    Style::default().fg(ACCENT)
}

/// Footer/help text style
pub fn footer() -> Style {
    Style::default().fg(MUTED)
}

/// Loading indicator style
pub fn loading() -> Style {
    Style::default().fg(HIGHLIGHT)
}

/// User message prefix style
pub fn user_prefix() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// AI message prefix style
pub fn ai_prefix() -> Style {
    Style::default().fg(SUCCESS)
}

/// System message style
pub fn system_prefix() -> Style {
    Style::default()
        .fg(HIGHLIGHT)
        .add_modifier(Modifier::ITALIC)
}

/// Mode badge - Chat mode
pub fn mode_chat() -> Style {
    Style::default().fg(Color::Black).bg(ACCENT)
}

/// Mode badge - Agent mode
pub fn mode_agent() -> Style {
    Style::default().fg(Color::Black).bg(SUCCESS)
}

/// Key hint style for help text
pub fn key_hint() -> Style {
    Style::default().fg(SUCCESS)
}

/// Destructive action hint
pub fn key_destructive() -> Style {
    Style::default().fg(ERROR)
}

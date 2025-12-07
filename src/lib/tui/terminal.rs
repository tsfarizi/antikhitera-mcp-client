//! Terminal setup and cleanup for Ratatui TUI

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, Stdout};

/// Type alias for our terminal
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize terminal for TUI mode
pub fn init_terminal() -> io::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore terminal to normal mode
pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Read a key event (blocking)
pub fn read_key() -> io::Result<KeyEvent> {
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                return Ok(key);
            }
        }
    }
}

/// Navigation action from key input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavAction {
    Up,
    Down,
    Select,
    Back,
    ForceQuit, // q or Ctrl+Q - exit entire app immediately
    None,
}

impl From<KeyEvent> for NavAction {
    fn from(key: KeyEvent) -> Self {
        use crossterm::event::KeyModifiers;

        // Ctrl+Q = Force quit entire app
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
            return NavAction::ForceQuit;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => NavAction::Up,
            KeyCode::Down | KeyCode::Char('j') => NavAction::Down,
            KeyCode::Enter | KeyCode::Char(' ') => NavAction::Select,
            KeyCode::Esc | KeyCode::Backspace => NavAction::Back,
            KeyCode::Char('q') => NavAction::ForceQuit, // q = exit entire app
            _ => NavAction::None,
        }
    }
}

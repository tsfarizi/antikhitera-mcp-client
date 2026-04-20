//! NavAction key mapping tests

use antikythera_core::tui::terminal::NavAction;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn make_key_with_ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

// Split into 5 parts for consistent test organization.
include!("nav_action_tests/part_01.rs");
include!("nav_action_tests/part_02.rs");
include!("nav_action_tests/part_03.rs");
include!("nav_action_tests/part_04.rs");
include!("nav_action_tests/part_05.rs");

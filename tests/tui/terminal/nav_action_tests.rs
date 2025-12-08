//! NavAction key mapping tests

use antikhitera_mcp_client::tui::terminal::NavAction;
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

#[test]
fn test_nav_action_up() {
    let action = NavAction::from(make_key(KeyCode::Up));
    assert_eq!(action, NavAction::Up);

    let action = NavAction::from(make_key(KeyCode::Char('k')));
    assert_eq!(action, NavAction::Up);
}

#[test]
fn test_nav_action_down() {
    let action = NavAction::from(make_key(KeyCode::Down));
    assert_eq!(action, NavAction::Down);

    let action = NavAction::from(make_key(KeyCode::Char('j')));
    assert_eq!(action, NavAction::Down);
}

#[test]
fn test_nav_action_select() {
    let action = NavAction::from(make_key(KeyCode::Enter));
    assert_eq!(action, NavAction::Select);

    let action = NavAction::from(make_key(KeyCode::Char(' ')));
    assert_eq!(action, NavAction::Select);
}

#[test]
fn test_nav_action_back() {
    let action = NavAction::from(make_key(KeyCode::Esc));
    assert_eq!(action, NavAction::Back);

    let action = NavAction::from(make_key(KeyCode::Backspace));
    assert_eq!(action, NavAction::Back);
}

#[test]
fn test_nav_action_force_quit() {
    let action = NavAction::from(make_key(KeyCode::Char('q')));
    assert_eq!(action, NavAction::ForceQuit);

    let action = NavAction::from(make_key_with_ctrl(KeyCode::Char('q')));
    assert_eq!(action, NavAction::ForceQuit);
}

#[test]
fn test_nav_action_none() {
    let action = NavAction::from(make_key(KeyCode::Char('x')));
    assert_eq!(action, NavAction::None);

    let action = NavAction::from(make_key(KeyCode::F(1)));
    assert_eq!(action, NavAction::None);
}

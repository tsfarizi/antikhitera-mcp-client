use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::presentation::tui::app::ChatApp;
use crate::presentation::tui::event_loop::KeyAction;

pub(crate) fn handle_history_key(key: KeyEvent, app: &mut ChatApp) -> KeyAction {
    // Rename mode intercepts all printable input.
    if app.history.rename_mode {
        match key.code {
            KeyCode::Esc => {
                app.history.rename_mode = false;
                app.history.rename_buffer.clear();
            }
            KeyCode::Enter => {
                let new_title = app.history.rename_buffer.trim().to_string();
                if !new_title.is_empty()
                    && let Some(id) = app
                        .history
                        .sessions
                        .get(app.history.cursor)
                        .map(|s| s.id.clone())
                    && app.history_store.rename_session(&id, new_title).is_ok()
                {
                    app.history.sessions = app.history_store.list_sessions();
                }
                app.history.rename_mode = false;
                app.history.rename_buffer.clear();
            }
            KeyCode::Backspace => {
                app.history.rename_buffer.pop();
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.history.rename_buffer.push(ch);
            }
            _ => {}
        }
        return KeyAction::None;
    }

    // Detail view — show full conversation, allow scrolling.
    if app.history.detail.is_some() {
        match key.code {
            KeyCode::Esc | KeyCode::Backspace => {
                app.history.detail = None;
                app.history.detail_scroll = 0;
            }
            KeyCode::Up => {
                app.history.detail_scroll = app.history.detail_scroll.saturating_sub(1);
            }
            KeyCode::Down => {
                app.history.detail_scroll += 1;
            }
            _ => {}
        }
        return KeyAction::None;
    }

    // List view — navigate, open, delete, rename.
    match key.code {
        KeyCode::Esc | KeyCode::F(3) => {
            app.history.open = false;
            app.status = "Siap.".to_string();
        }
        KeyCode::Up => {
            app.history.cursor = app.history.cursor.saturating_sub(1);
        }
        KeyCode::Down => {
            let max = app.history.sessions.len().saturating_sub(1);
            if app.history.cursor < max {
                app.history.cursor += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(id) = app
                .history
                .sessions
                .get(app.history.cursor)
                .map(|s| s.id.clone())
            {
                app.history.detail = app.history_store.load_session(&id);
                app.history.detail_scroll = 0;
            }
        }
        KeyCode::Char('d') => {
            if let Some(id) = app
                .history
                .sessions
                .get(app.history.cursor)
                .map(|s| s.id.clone())
            {
                let _ = app.history_store.delete_session(&id);
                app.history.sessions = app.history_store.list_sessions();
                let max = app.history.sessions.len().saturating_sub(1);
                app.history.cursor = app.history.cursor.min(max);
            }
        }
        KeyCode::Char('r') => {
            let buf = app
                .history
                .sessions
                .get(app.history.cursor)
                .map(|s| s.title.clone())
                .unwrap_or_default();
            app.history.rename_buffer = buf;
            app.history.rename_mode = true;
        }
        _ => {}
    }
    KeyAction::None
}

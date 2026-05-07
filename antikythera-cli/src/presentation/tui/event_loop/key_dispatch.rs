use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::app::ChatApp;
use super::super::handlers::history_handler::handle_history_key;
use super::super::handlers::settings_handler::handle_settings_key;
use super::result_handler::scroll_to_bottom;

pub(crate) enum KeyAction {
    None,
    Submit,
    ApplySettings,
    Quit,
}

pub(super) fn handle_key_event(key: KeyEvent, app: &mut ChatApp) -> KeyAction {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        return KeyAction::Quit;
    }

    // Route all input to settings panel when it's open.
    if app.settings.open {
        return handle_settings_key(key, app);
    }

    // Route all input to history browser when it's open.
    if app.history.open {
        return handle_history_key(key, app);
    }

    // F2 opens the full settings panel.
    if key.code == KeyCode::F(2) {
        let provider = app.provider.clone();
        let model = app.model.clone();
        let agent_mode = app.agent_mode;
        let config = app.runtime_config.clone();
        let providers = app.providers.clone();
        app.settings
            .open_with(&provider, &model, &config, &providers, agent_mode);
        app.status =
            "Settings terbuka. Tab/BackTab=ganti tab | ↑↓=navigasi | Enter=pilih | Ctrl+S=simpan | Esc=tutup".to_string();
        return KeyAction::None;
    }

    // F3 opens the history browser.
    if key.code == KeyCode::F(3) {
        let sessions = app.history_store.list_sessions();
        app.history.open_and_refresh_with(sessions);
        app.status =
            "Riwayat Chat. ↑↓=navigasi | Enter=lihat | d=hapus | r=ganti judul | Esc=tutup"
                .to_string();
        return KeyAction::None;
    }

    match key.code {
        KeyCode::Esc => KeyAction::Quit,
        KeyCode::Enter => KeyAction::Submit,
        KeyCode::Backspace => {
            app.input.pop();
            KeyAction::None
        }
        KeyCode::Tab => {
            if let Some((command, _)) = app.suggestions().first() {
                app.input = format!("/{command}");
            }
            KeyAction::None
        }
        // ── LOG scroll (Ctrl + arrows) ───────────────────────────────────
        // Guarded arms must come BEFORE the unguarded arrow-key arms.
        KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.log_scroll = app.log_scroll.saturating_sub(3);
            KeyAction::None
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.log_scroll = app.log_scroll.saturating_add(3);
            KeyAction::None
        }
        KeyCode::PageUp if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.log_scroll = app.log_scroll.saturating_sub(20);
            KeyAction::None
        }
        KeyCode::PageDown if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.log_scroll = app.log_scroll.saturating_add(20);
            KeyAction::None
        }
        KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.log_scroll = 0;
            KeyAction::None
        }
        KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let total = app.log_lines.len().saturating_sub(12);
            app.log_scroll = total as u16;
            KeyAction::None
        }
        // ── Conversation scroll ──────────────────────────────────────────
        KeyCode::Up => {
            app.conversation_scroll = app.conversation_scroll.saturating_sub(3);
            KeyAction::None
        }
        KeyCode::Down => {
            app.conversation_scroll = app.conversation_scroll.saturating_add(3);
            KeyAction::None
        }
        KeyCode::PageUp => {
            app.conversation_scroll = app.conversation_scroll.saturating_sub(20);
            KeyAction::None
        }
        KeyCode::PageDown => {
            app.conversation_scroll = app.conversation_scroll.saturating_add(20);
            KeyAction::None
        }
        KeyCode::Home => {
            app.conversation_scroll = 0;
            KeyAction::None
        }
        KeyCode::End => {
            app.conversation_scroll = scroll_to_bottom(&app.messages, app.conversation_scroll);
            KeyAction::None
        }
        KeyCode::Char(character) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.input.push(character);
            }
            KeyAction::None
        }
        _ => KeyAction::None,
    }
}

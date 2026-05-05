use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::infrastructure::llm::ModelInfo;
use crate::presentation::tui::app::ChatApp;
use crate::presentation::tui::event_loop::KeyAction;
use crate::presentation::tui::types::{PromptField, SettingsTab};

pub(crate) fn handle_settings_key(key: KeyEvent, app: &mut ChatApp) -> KeyAction {
    // Ctrl+S — save all pending changes and close.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        app.settings.model_add_mode = false;
        app.settings.model_add_buffer.clear();
        app.settings.open = false;
        return KeyAction::ApplySettings;
    }

    // ── Model "add" input mode ───────────────────────────────────────────────
    // Intercept keystrokes while the user is typing a new model name.
    if app.settings.model_add_mode {
        match key.code {
            KeyCode::Esc => {
                app.settings.model_add_mode = false;
                app.settings.model_add_buffer.clear();
            }
            KeyCode::Enter => {
                let name = app.settings.model_add_buffer.trim().to_string();
                if !name.is_empty()
                    && let Some(provider) = app.providers.get_mut(app.settings.pending_provider_idx)
                    && !provider.models.iter().any(|m| m.name == name)
                {
                    provider.models.push(ModelInfo {
                        name,
                        display_name: None,
                    });
                    // Move cursor to the newly added model.
                    app.settings.model_cursor = provider.models.len().saturating_sub(1);
                }
                app.settings.model_add_mode = false;
                app.settings.model_add_buffer.clear();
            }
            KeyCode::Backspace => {
                app.settings.model_add_buffer.pop();
            }
            KeyCode::Char(ch)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && (ch.is_alphanumeric() || matches!(ch, '-' | '.' | '_' | ':')) =>
            {
                app.settings.model_add_buffer.push(ch);
            }
            _ => {}
        }
        return KeyAction::None;
    }

    // While in text-edit mode, route keystrokes to the edit buffer.
    if app.settings.editing {
        match key.code {
            KeyCode::Esc => {
                app.settings.editing = false;
                app.settings.edit_buffer.clear();
            }
            // Ctrl+Enter commits the field edit.
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let value = std::mem::take(&mut app.settings.edit_buffer);
                commit_settings_edit(app, value);
                app.settings.editing = false;
            }
            KeyCode::Enter => {
                app.settings.edit_buffer.push('\n');
            }
            KeyCode::Backspace => {
                app.settings.edit_buffer.pop();
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.settings.edit_buffer.push(ch);
            }
            _ => {}
        }
        return KeyAction::None;
    }

    // Esc closes settings panel without applying.
    if key.code == KeyCode::Esc {
        app.settings.open = false;
        return KeyAction::None;
    }

    match key.code {
        KeyCode::Tab => {
            app.settings.tab = app.settings.tab.next();
        }
        KeyCode::BackTab => {
            app.settings.tab = app.settings.tab.prev();
        }
        // Number shortcut keys (1-5) for direct tab jump.
        KeyCode::Char('1') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.settings.tab = SettingsTab::Provider;
        }
        KeyCode::Char('2') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.settings.tab = SettingsTab::Model;
        }
        KeyCode::Char('3') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.settings.tab = SettingsTab::Prompts;
        }
        KeyCode::Char('4') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.settings.tab = SettingsTab::System;
        }
        KeyCode::Char('5') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.settings.tab = SettingsTab::Agent;
        }
        KeyCode::Up => match app.settings.tab {
            SettingsTab::Provider => {
                app.settings.provider_cursor = app.settings.provider_cursor.saturating_sub(1);
            }
            SettingsTab::Model => {
                app.settings.model_cursor = app.settings.model_cursor.saturating_sub(1);
            }
            SettingsTab::Prompts => {
                app.settings.prompt_cursor = app.settings.prompt_cursor.saturating_sub(1);
            }
            _ => {}
        },
        KeyCode::Down => match app.settings.tab {
            SettingsTab::Provider => {
                let max = app.providers.len().saturating_sub(1);
                if app.settings.provider_cursor < max {
                    app.settings.provider_cursor += 1;
                }
            }
            SettingsTab::Model => {
                let max = app
                    .providers
                    .get(app.settings.pending_provider_idx)
                    .map(|p| p.models.len().saturating_sub(1))
                    .unwrap_or(0);
                if app.settings.model_cursor < max {
                    app.settings.model_cursor += 1;
                }
            }
            SettingsTab::Prompts if app.settings.prompt_cursor + 1 < PromptField::COUNT => {
                app.settings.prompt_cursor += 1;
            }
            _ => {}
        },
        KeyCode::Enter => match app.settings.tab {
            SettingsTab::Provider => {
                app.settings.pending_provider_idx = app.settings.provider_cursor;
                app.settings.pending_model_idx = 0;
                app.settings.model_cursor = 0;
                // Jump to Model tab so user can pick the model.
                app.settings.tab = SettingsTab::Model;
            }
            SettingsTab::Model => {
                app.settings.pending_model_idx = app.settings.model_cursor;
            }
            SettingsTab::Prompts => {
                if let Some(&field) = PromptField::ALL.get(app.settings.prompt_cursor) {
                    app.settings.edit_buffer = field.get_from(&app.settings.pending_prompts);
                    app.settings.editing = true;
                }
            }
            SettingsTab::System => {
                app.settings.edit_buffer = app.settings.pending_system_prompt.clone();
                app.settings.editing = true;
            }
            SettingsTab::Agent => {
                app.settings.pending_agent_mode = !app.settings.pending_agent_mode;
            }
        },
        // ── Add model (Model tab only) ────────────────────────────────────────
        KeyCode::Char('a') if app.settings.tab == SettingsTab::Model => {
            app.settings.model_add_mode = true;
            app.settings.model_add_buffer.clear();
        }
        // ── Delete model (Model tab only) ─────────────────────────────────────
        KeyCode::Char('d') if app.settings.tab == SettingsTab::Model => {
            let idx = app.settings.pending_provider_idx;
            let cursor = app.settings.model_cursor;
            if let Some(provider) = app.providers.get_mut(idx)
                && cursor < provider.models.len()
            {
                provider.models.remove(cursor);
                let new_len = provider.models.len();
                // Keep cursors in bounds after removal.
                app.settings.model_cursor = cursor.min(new_len.saturating_sub(1));
                app.settings.pending_model_idx = app
                    .settings
                    .pending_model_idx
                    .min(new_len.saturating_sub(1));
            }
        }
        _ => {}
    }

    KeyAction::None
}

fn commit_settings_edit(app: &mut ChatApp, value: String) {
    match app.settings.tab {
        SettingsTab::System => {
            app.settings.pending_system_prompt = value;
        }
        SettingsTab::Prompts => {
            if let Some(&field) = PromptField::ALL.get(app.settings.prompt_cursor) {
                field.set_into(&mut app.settings.pending_prompts, value);
            }
        }
        _ => {}
    }
}

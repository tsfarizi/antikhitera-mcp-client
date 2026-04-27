use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::config::{
    AppConfig as PostcardAppConfig, ModelConfig as PostcardModelConfig, save_app_config,
};
use crate::infrastructure::llm::{ModelInfo, ModelProviderConfig, providers_to_postcard};
use crate::infrastructure::llm::{StreamEvent, clear_stream_event_sink, set_stream_event_sink};
use antikythera_core::application::agent::{Agent, AgentOptions, AgentOutcome, AgentStep};
use antikythera_core::application::client::{ChatRequest, ChatResult, McpClient, McpError};
use antikythera_core::application::resilience::{ContextWindowPolicy, RetryPolicy, with_retry_if};
use antikythera_core::config::AppConfig;
use antikythera_core::get_latest_logs;
use antikythera_core::infrastructure::model::DynamicModelProvider;
use antikythera_sdk::sdk_logging::get_latest_sdk_logs;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::{mpsc, oneshot};

use crate::CliResult;
use crate::infrastructure::history::{ChatHistorySession, ChatHistoryStore, ChatTurn, TurnRole};
use crate::runtime::{build_runtime_client, materialize_runtime_config};

use super::app::ChatApp;
use super::render::draw;
use super::types::{
    PendingResponse, PromptField, SLASH_COMMANDS, SettingsTab, UiMessage, UiTone,
    slash_command_suggestions,
};

pub async fn run_chat_app(
    mut config: AppConfig,
    providers: Vec<ModelProviderConfig>,
) -> CliResult<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run server auto-discovery before building the client so any discovered
    // servers are included in the ServerManager that the client owns.
    let discovery_msg = {
        use antikythera_core::application::discovery::startup::run_startup_discovery;
        use antikythera_core::config::server::{ServerConfig as CoreServerConfig, TransportType};
        use std::collections::HashMap;

        let result = run_startup_discovery(None).await;
        let mut added = 0usize;
        for server in result.loaded_servers() {
            let sc = CoreServerConfig {
                name: server.name.clone(),
                transport: TransportType::Stdio,
                command: Some(server.binary_path.clone()),
                args: Vec::new(),
                env: HashMap::new(),
                workdir: None,
                url: None,
                headers: HashMap::new(),
                default_timezone: None,
                default_city: None,
            };
            if !config.servers.iter().any(|s| s.name == sc.name) {
                config.servers.push(sc);
                added += 1;
            }
        }
        if result.folder_exists {
            Some(format!(
                "Server discovery: {} ditemukan, {} berhasil diload, {} ditambahkan ke konfigurasi aktif.",
                result.summary.total_found, result.summary.loaded, added
            ))
        } else {
            None // servers/ folder not found — discovery is optional; omit to avoid noise at startup.
        }
    };

    let client = build_runtime_client(&config, &providers)?;
    let snapshot = client.config_snapshot();
    let tools = client.tools().len();
    let mut app = ChatApp::new(config, providers, snapshot, tools);
    if let Some(msg) = discovery_msg {
        app.push_message(UiMessage::new("Server Discovery", msg, UiTone::System));
    }
    let result = run_loop(&mut terminal, client, app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut client: Arc<McpClient<DynamicModelProvider>>,
    mut app: ChatApp,
) -> CliResult<()> {
    loop {
        // Drain live-streaming token chunks into the preview buffer.
        if let Some(rx) = &mut app.stream_rx {
            while let Ok(chunk) = rx.try_recv() {
                app.streaming_content.push_str(&chunk);
            }
        }

        // Poll for a completed in-flight request spawned in a previous iteration.
        if let Some(mut rx) = app.pending_rx.take() {
            use tokio::sync::oneshot::error::TryRecvError;
            match rx.try_recv() {
                Ok(PendingResponse::Chat(Ok(result))) => {
                    app.loading = false;
                    app.streaming_content.clear();
                    app.stream_rx = None;
                    clear_stream_event_sink();
                    apply_chat_result(&mut app, result);
                }
                Ok(PendingResponse::Chat(Err(msg))) => {
                    app.loading = false;
                    app.streaming_content.clear();
                    app.stream_rx = None;
                    clear_stream_event_sink();
                    app.status = "Model gagal menjawab.".to_string();
                    app.push_message(UiMessage::new("Model Error", msg, UiTone::Error));
                }
                Ok(PendingResponse::Agent(Ok(outcome))) => {
                    app.loading = false;
                    app.streaming_content.clear();
                    app.stream_rx = None;
                    clear_stream_event_sink();
                    apply_agent_outcome(&mut app, outcome);
                }
                Ok(PendingResponse::Agent(Err(msg))) => {
                    app.loading = false;
                    app.streaming_content.clear();
                    app.stream_rx = None;
                    clear_stream_event_sink();
                    app.status = "Agent gagal menyelesaikan permintaan.".to_string();
                    app.push_message(UiMessage::new("Agent Error", msg, UiTone::Error));
                }
                Err(TryRecvError::Empty) => {
                    // Task still running — put the receiver back.
                    app.pending_rx = Some(rx);
                }
                Err(TryRecvError::Closed) => {
                    app.loading = false;
                    app.streaming_content.clear();
                    app.stream_rx = None;
                    clear_stream_event_sink();
                    app.status =
                        "Kesalahan internal: proses respons berhenti tidak terduga.".to_string();
                }
            }
        }

        // Refresh log panel from both core and SDK logging systems on every frame tick.
        // Core logs: transport (data transfer), provider (API calls), agent (tool calls).
        // SDK logs: FFI calls, WasmAgent events, config/server operations.
        // The tracing bridge (AntikytheraTuiLayer) writes all tracing:: events to the
        // "tui" session bucket — always read from "tui" here regardless of chat session.
        let mut core_logs = get_latest_logs("tui", 50);
        let mut sdk_logs = get_latest_sdk_logs("tui", 20);
        // Merge: tag SDK entries so they can be highlighted differently.
        // We encode source prefix "sdk:" to differentiate from core sources.
        for entry in &mut sdk_logs {
            if let Some(ref mut src) = entry.source {
                *src = format!("sdk:{src}");
            } else {
                entry.source = Some("sdk:ffi".to_string());
            }
        }
        core_logs.extend(sdk_logs);
        // Sort by sequence number for correct chronological order.
        core_logs.sort_by_key(|e| e.sequence);
        app.log_lines = core_logs
            .into_iter()
            .rev()
            .map(|entry| {
                let level = entry.level.as_str();
                let source = entry.source.as_deref().unwrap_or("core");
                // Include context payload (FFI args, tool names, etc.) when present.
                if let Some(ctx) = &entry.context {
                    format!("[{level:<5}][{source}] {} | {ctx}", entry.message)
                } else {
                    format!("[{level:<5}][{source}] {}", entry.message)
                }
            })
            .collect();

        terminal.draw(|frame| draw(frame, &app))?;

        if app.should_quit {
            break;
        }

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            // On Windows crossterm emits both Press and Release events.
            // Only process Press (and Repeat for held keys) to avoid doubles.
            if key.kind == crossterm::event::KeyEventKind::Release {
                continue;
            }
            match handle_key_event(key, &mut app) {
                KeyAction::None => {}
                KeyAction::Submit => {
                    submit_input(&mut client, &mut app);
                }
                KeyAction::ApplySettings => {
                    // Extract pending provider / model from the settings panel.
                    let provider_id = app
                        .providers
                        .get(app.settings.pending_provider_idx)
                        .map(|p| p.id.clone())
                        .unwrap_or_else(|| app.provider.clone());
                    let model_name = app
                        .providers
                        .get(app.settings.pending_provider_idx)
                        .and_then(|p| p.models.get(app.settings.pending_model_idx))
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| app.model.clone());

                    // Write pending prompts + system prompt back into runtime config.
                    app.runtime_config.prompts = app.settings.pending_prompts.clone();
                    app.runtime_config.system_prompt =
                        if app.settings.pending_system_prompt.is_empty() {
                            None
                        } else {
                            Some(app.settings.pending_system_prompt.clone())
                        };
                    app.agent_mode = app.settings.pending_agent_mode;

                    match apply_runtime_selection(&mut app, provider_id, model_name) {
                        Ok(_) => {
                            if let Err(err) = reconfigure_runtime(&mut app, &mut client) {
                                app.status = format!("Gagal menerapkan settings: {}", err);
                            } else {
                                app.status = format!(
                                    "Settings disimpan. Provider: {}, Model: {}.",
                                    app.provider, app.model
                                );
                            }
                        }
                        Err(err) => {
                            app.status = format!("Gagal menerapkan settings: {}", err);
                        }
                    }
                }
                KeyAction::Quit => app.should_quit = true,
            }
        }
    }

    Ok(())
}

enum KeyAction {
    None,
    Submit,
    ApplySettings,
    Quit,
}

fn handle_key_event(key: KeyEvent, app: &mut ChatApp) -> KeyAction {
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
        KeyCode::Char(character) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.input.push(character);
            }
            KeyAction::None
        }
        _ => KeyAction::None,
    }
}

// ── History browser key handler ───────────────────────────────────────────────

fn handle_history_key(key: KeyEvent, app: &mut ChatApp) -> KeyAction {
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

fn handle_settings_key(key: KeyEvent, app: &mut ChatApp) -> KeyAction {
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

fn submit_input(client: &mut Arc<McpClient<DynamicModelProvider>>, app: &mut ChatApp) {
    let input = app.input.trim().to_string();
    app.input.clear();

    if input.is_empty() {
        app.status = "Ketik pesan atau slash command untuk melanjutkan.".to_string();
        return;
    }

    // Prevent double-submission while a request is already in-flight.
    if app.pending_rx.is_some() {
        app.status = "Menunggu respons...".to_string();
        return;
    }

    if input.starts_with('/') {
        process_command(app, client, &input);
        return;
    }

    app.push_message(UiMessage::new("You", &input, UiTone::User));
    app.status = format!("Mengirim ke {}/{}...", app.provider, app.model);
    app.loading = true;

    // Capture user turn into the in-flight debug history session.
    if app.current_history_session.is_none() {
        app.current_history_session = Some(ChatHistorySession::new(
            ChatHistoryStore::new_id(),
            app.provider.clone(),
            app.model.clone(),
            app.agent_mode,
        ));
    }
    if let Some(session) = &mut app.current_history_session {
        if let Some(ref id) = app.session_id {
            session.core_session_id = Some(id.clone());
        }
        session.turns.push(ChatTurn {
            timestamp: Utc::now().to_rfc3339(),
            role: TurnRole::User,
            content: input.clone(),
            tool_steps: 0,
        });
    }

    let (tx, rx) = oneshot::channel();
    app.pending_rx = Some(rx);

    // Install a streaming sink that forwards token chunks to the TUI render loop
    // so tokens appear live in the Conversation panel while the task runs.
    let (stream_tx, stream_rx) = mpsc::unbounded_channel::<String>();
    app.stream_rx = Some(stream_rx);
    app.streaming_content.clear();
    set_stream_event_sink(Arc::new(move |event: &StreamEvent| {
        if let StreamEvent::Chunk { content, .. } = event {
            let _ = stream_tx.send(content.clone());
        }
    }));

    let health_ref = Arc::clone(&app.health);
    let provider_id = app.provider.clone();

    if app.agent_mode {
        let options = AgentOptions {
            session_id: app.session_id.clone(),
            ..AgentOptions::default()
        };
        let client_arc = Arc::clone(client);
        tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = Agent::new(client_arc)
                .run(input, options)
                .await
                .map_err(|e| e.user_message());
            let elapsed_ms = start.elapsed().as_millis() as u64;
            if let Ok(mut h) = health_ref.lock() {
                match &result {
                    Ok(_) => h.record_success(&provider_id, elapsed_ms),
                    Err(e) => h.record_failure(&provider_id, e.as_str()),
                }
            }
            let _ = tx.send(PendingResponse::Agent(result));
        });
    } else {
        let client_arc = Arc::clone(client);
        let session_id = app.session_id.clone();
        let cw_policy = ContextWindowPolicy::default();
        let retry_policy = RetryPolicy::default();
        tokio::spawn(async move {
            // Auto-prune context window before sending if the session is long.
            if let Some(ref sid) = session_id {
                let removed = client_arc.prune_session(sid, &cw_policy).await;
                if removed > 0 {
                    tracing::info!(removed, "Context window pruned before request");
                }
            }

            let start = std::time::Instant::now();
            // Retry on transient failures with exponential back-off.
            let result: Result<ChatResult, McpError> = with_retry_if(
                &retry_policy,
                || {
                    let c = Arc::clone(&client_arc);
                    let prompt = input.clone();
                    let sid = session_id.clone();
                    async move {
                        c.chat(ChatRequest {
                            prompt,
                            attachments: Vec::new(),
                            system_prompt: None,
                            session_id: sid,
                            raw_mode: false,
                            bypass_template: false,
                            force_json: false,
                        })
                        .await
                    }
                },
                |_: &McpError| true,
            )
            .await;

            let elapsed_ms = start.elapsed().as_millis() as u64;
            if let Ok(mut h) = health_ref.lock() {
                match &result {
                    Ok(r) => h.record_success(&r.provider, elapsed_ms),
                    Err(e) => h.record_failure(&provider_id, e.user_message()),
                }
            }
            let _ = tx.send(PendingResponse::Chat(result.map_err(|e| e.user_message())));
        });
    }
}

fn apply_chat_result(app: &mut ChatApp, result: ChatResult) {
    app.session_id = Some(result.session_id.clone());
    app.status = format!(
        "Respons diterima dari {}/{}.",
        result.provider, result.model
    );
    app.push_message(UiMessage::new(
        format!("Assistant [{}]", result.provider),
        result.content.clone(),
        UiTone::Assistant,
    ));

    // Append assistant turn and persist the debug history session.
    if let Some(session) = &mut app.current_history_session {
        session.core_session_id = Some(result.session_id.clone());
        session.updated_at = Utc::now().to_rfc3339();
        if session.title.is_empty()
            && let Some(first) = session.turns.iter().find(|t| t.role == TurnRole::User)
        {
            session.title = first.content.chars().take(60).collect();
        }
        session.turns.push(ChatTurn {
            timestamp: Utc::now().to_rfc3339(),
            role: TurnRole::Assistant,
            content: result.content.clone(),
            tool_steps: 0,
        });
        let _ = app.history_store.save_session(session);
    }
}

fn apply_agent_outcome(app: &mut ChatApp, outcome: AgentOutcome) {
    app.session_id = Some(outcome.session_id.clone());
    app.status = format!("Agent selesai dengan {} langkah tool.", outcome.steps.len());
    let response_text = format_agent_response(&outcome.response);
    app.push_message(UiMessage::new(
        "Agent",
        response_text.clone(),
        UiTone::Assistant,
    ));

    if !outcome.steps.is_empty() {
        app.push_message(UiMessage::new(
            "Tool Trace",
            render_steps_summary(&outcome.steps),
            UiTone::System,
        ));
    }

    // Append assistant turn and persist the debug history session.
    let tool_step_count = outcome.steps.len();
    if let Some(session) = &mut app.current_history_session {
        session.core_session_id = Some(outcome.session_id.clone());
        session.updated_at = Utc::now().to_rfc3339();
        if session.title.is_empty()
            && let Some(first) = session.turns.iter().find(|t| t.role == TurnRole::User)
        {
            session.title = first.content.chars().take(60).collect();
        }
        session.turns.push(ChatTurn {
            timestamp: Utc::now().to_rfc3339(),
            role: TurnRole::Assistant,
            content: response_text,
            tool_steps: tool_step_count,
        });
        let _ = app.history_store.save_session(session);
    }
}

fn process_command(
    app: &mut ChatApp,
    client: &mut Arc<McpClient<DynamicModelProvider>>,
    input: &str,
) {
    let command = input.trim_start_matches('/').trim();
    let mut parts = command.split_whitespace();
    let name = parts.next().unwrap_or_default().to_ascii_lowercase();
    let args: Vec<&str> = parts.collect();

    match name.as_str() {
        "help" | "?" => {
            app.status = "Bantuan command diperbarui di panel chat.".to_string();
            app.push_message(UiMessage::new(
                "Slash Commands",
                SLASH_COMMANDS
                    .iter()
                    .map(|(name, description)| format!("/{name:<10} {description}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                UiTone::System,
            ));
        }
        "providers" | "provider" => {
            app.status = "Daftar provider tersedia ditampilkan.".to_string();
            app.push_message(UiMessage::new(
                "Providers",
                render_provider_catalog(&app.providers, &app.provider, &app.model),
                UiTone::System,
            ));
        }
        "use" => {
            let Some(provider_input) = args.first().copied() else {
                app.push_message(UiMessage::new(
                    "Command Error",
                    "Gunakan /use <provider> [model]. Contoh: /use openai gpt-4o-mini",
                    UiTone::Error,
                ));
                return;
            };

            match apply_provider_selection(app, provider_input, args.get(1).copied()) {
                Ok(message) => {
                    if let Err(error) = reconfigure_runtime(app, client) {
                        app.status = "Gagal menerapkan backend runtime.".to_string();
                        app.push_message(UiMessage::new("Command Error", error, UiTone::Error));
                        return;
                    }
                    app.status = format!(
                        "Backend aktif diperbarui ke {}/{}.",
                        app.provider, app.model
                    );
                    app.push_message(UiMessage::new("Runtime Updated", message, UiTone::System));
                }
                Err(error) => {
                    app.status = "Gagal mengganti provider/model.".to_string();
                    app.push_message(UiMessage::new("Command Error", error, UiTone::Error));
                }
            }
        }
        "model" => {
            let Some(model_input) = args.first().copied() else {
                app.push_message(UiMessage::new(
                    "Command Error",
                    "Gunakan /model <nama-model>. Contoh: /model gemini-2.0-flash",
                    UiTone::Error,
                ));
                return;
            };

            match apply_model_selection(app, model_input) {
                Ok(message) => {
                    if let Err(error) = reconfigure_runtime(app, client) {
                        app.status = "Gagal menerapkan backend runtime.".to_string();
                        app.push_message(UiMessage::new("Command Error", error, UiTone::Error));
                        return;
                    }
                    app.status = format!("Model aktif diperbarui ke {}.", app.model);
                    app.push_message(UiMessage::new("Runtime Updated", message, UiTone::System));
                }
                Err(error) => {
                    app.status = "Gagal mengganti model aktif.".to_string();
                    app.push_message(UiMessage::new("Command Error", error, UiTone::Error));
                }
            }
        }
        "config" => {
            app.status = "Ringkasan konfigurasi aktif ditampilkan.".to_string();
            app.push_message(UiMessage::new(
                "Config Snapshot",
                render_config_snapshot(&app.snapshot),
                UiTone::System,
            ));
        }
        "tools" => {
            let body = if client.tools().is_empty() {
                "Tidak ada tool yang aktif pada sesi ini.".to_string()
            } else {
                client
                    .tools()
                    .iter()
                    .map(|tool| format!("- {}", tool.name))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            app.status = "Daftar tools aktif ditampilkan.".to_string();
            app.push_message(UiMessage::new("Tools", body, UiTone::System));
        }
        "agent" => {
            let next_mode = match args.first().copied() {
                Some("on") => true,
                Some("off") => false,
                Some("toggle") | None => !app.agent_mode,
                Some(other) => {
                    app.push_message(UiMessage::new(
                        "Command Error",
                        format!(
                            "Argumen /agent '{}' tidak dikenal. Gunakan on, off, atau toggle.",
                            other
                        ),
                        UiTone::Error,
                    ));
                    return;
                }
            };
            app.agent_mode = next_mode;
            app.status = if next_mode {
                "Mode agent aktif.".to_string()
            } else {
                "Mode chat langsung aktif.".to_string()
            };
            app.push_message(UiMessage::new(
                "Mode Updated",
                if next_mode {
                    "Prompt berikutnya akan dieksekusi melalui loop agent."
                } else {
                    "Prompt berikutnya akan langsung dikirim ke model tanpa loop agent."
                },
                UiTone::System,
            ));
        }
        "reset" | "clear" => app.reset_session(),
        "history" => {
            let sessions = app.history_store.list_sessions();
            app.history.open_and_refresh_with(sessions);
            app.status =
                "Riwayat Chat. ↑↓=navigasi | Enter=lihat | d=hapus | r=ganti judul | Esc=tutup"
                    .to_string();
        }
        "exit" | "quit" => {
            app.status = "Menutup TUI...".to_string();
            app.should_quit = true;
        }
        other => {
            app.status = "Command tidak dikenal.".to_string();
            let suggestion_text = slash_command_suggestions(&format!("/{other}"))
                .into_iter()
                .map(|(name, description)| format!("/{name} - {description}"))
                .collect::<Vec<_>>()
                .join("\n");
            let body = if suggestion_text.is_empty() {
                format!("Perintah '/{other}' tidak dikenal. Gunakan /help untuk daftar command.")
            } else {
                format!(
                    "Perintah '/{other}' tidak dikenal. Mungkin yang Anda maksud:\n{}",
                    suggestion_text
                )
            };
            app.push_message(UiMessage::new("Command Error", body, UiTone::Error));
        }
    }
}

fn format_agent_response(value: &serde_json::Value) -> String {
    value.as_str().map(ToOwned::to_owned).unwrap_or_else(|| {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    })
}

fn render_steps_summary(steps: &[AgentStep]) -> String {
    steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            format!(
                "{}. {} [{}]{}",
                index + 1,
                step.tool,
                if step.success { "ok" } else { "failed" },
                step.message
                    .as_deref()
                    .map(|message| format!(" - {}", message))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_provider_catalog(
    providers: &[ModelProviderConfig],
    active_provider: &str,
    active_model: &str,
) -> String {
    providers
        .iter()
        .map(|provider| {
            let marker = if provider.id == active_provider {
                "*"
            } else {
                " "
            };
            let models = provider
                .models
                .iter()
                .map(|model| {
                    if provider.id == active_provider && model.name == active_model {
                        format!("{} (aktif)", model.name)
                    } else {
                        model.name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{marker} {} [{}]\n  endpoint: {}\n  models  : {}",
                provider.id, provider.provider_type, provider.endpoint, models
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_config_snapshot(
    snapshot: &antikythera_core::application::client::ClientConfigSnapshot,
) -> String {
    [
        format!("default provider : {}", snapshot.default_provider),
        format!("default model    : {}", snapshot.model),
        format!(
            "system prompt    : {}",
            snapshot.system_prompt.as_deref().unwrap_or("<none>")
        ),
        format!("servers          : {}", snapshot.servers.len()),
        format!("tools            : {}", snapshot.tools.len()),
        format!("template chars   : {}", snapshot.prompt_template.len()),
    ]
    .join("\n")
}

fn apply_provider_selection(
    app: &mut ChatApp,
    provider_input: &str,
    model_input: Option<&str>,
) -> Result<String, String> {
    let (provider, model) = resolve_provider_selection(
        &app.providers,
        &app.provider,
        &app.model,
        provider_input,
        model_input,
    )?;
    apply_runtime_selection(app, provider, model)
}

fn apply_model_selection(app: &mut ChatApp, model_input: &str) -> Result<String, String> {
    apply_runtime_selection(app, app.provider.clone(), model_input.trim().to_string())
}

fn apply_runtime_selection(
    app: &mut ChatApp,
    provider: String,
    model: String,
) -> Result<String, String> {
    let (updated_config, updated_providers) = materialize_runtime_config(
        &app.runtime_config,
        &app.providers,
        Some(&provider),
        Some(&model),
        None,
        None,
        app.runtime_config.system_prompt.as_deref(),
    )
    .map_err(|error| error.to_string())?;

    app.runtime_config = updated_config;
    app.providers = updated_providers;
    app.provider = app.runtime_config.default_provider.clone();
    app.model = app.runtime_config.model.clone();
    app.session_id = None;
    // Provider/model changed — start a fresh history session next turn.
    app.current_history_session = None;

    Ok(format!(
        "Provider/model aktif sekarang {}/{}. Sesi percakapan direset agar riwayat tidak tercampur antar backend.",
        app.provider, app.model
    ))
}

fn resolve_provider_selection(
    providers: &[ModelProviderConfig],
    current_provider: &str,
    current_model: &str,
    provider_input: &str,
    model_input: Option<&str>,
) -> Result<(String, String), String> {
    let provider = find_provider(providers, provider_input).ok_or_else(|| {
        format!(
            "Provider '{}' tidak ditemukan. Gunakan /providers untuk melihat backend yang tersedia.",
            provider_input.trim()
        )
    })?;

    let fallback_model =
        if provider.id.eq_ignore_ascii_case(current_provider) && !current_model.trim().is_empty() {
            current_model.trim().to_string()
        } else {
            provider
                .models
                .first()
                .map(|candidate| candidate.name.clone())
                .unwrap_or_else(|| current_model.trim().to_string())
        };

    let model = model_input
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(fallback_model);

    if model.is_empty() {
        return Err(format!(
            "Provider '{}' belum memiliki model default. Tambahkan nama model secara eksplisit, misalnya /use {} <model>.",
            provider.id, provider.id
        ));
    }

    Ok((provider.id.clone(), model))
}

fn reconfigure_runtime(
    app: &mut ChatApp,
    client: &mut Arc<McpClient<DynamicModelProvider>>,
) -> Result<(), String> {
    // Build a PostcardAppConfig to persist — merge core routing fields with CLI providers.
    // Convert runtime PromptsConfig (Option<String> fields) to the postcard form (String fields).
    let postcard_prompts = {
        use crate::config::PromptsConfig as PcPrompts;
        let defaults = PcPrompts::default();
        let r = &app.runtime_config.prompts;
        PcPrompts {
            template: r.template.clone().unwrap_or(defaults.template),
            tool_guidance: r.tool_guidance.clone().unwrap_or(defaults.tool_guidance),
            fallback_guidance: r
                .fallback_guidance
                .clone()
                .unwrap_or(defaults.fallback_guidance),
            json_retry_message: r
                .json_retry_message
                .clone()
                .unwrap_or(defaults.json_retry_message),
            tool_result_instruction: r
                .tool_result_instruction
                .clone()
                .unwrap_or(defaults.tool_result_instruction),
            agent_instructions: r
                .agent_instructions
                .clone()
                .unwrap_or(defaults.agent_instructions),
            ui_instructions: r
                .ui_instructions
                .clone()
                .unwrap_or(defaults.ui_instructions),
            language_instructions: r
                .language_instructions
                .clone()
                .unwrap_or(defaults.language_instructions),
            agent_max_steps_error: r
                .agent_max_steps_error
                .clone()
                .unwrap_or(defaults.agent_max_steps_error),
            no_tools_guidance: r
                .no_tools_guidance
                .clone()
                .unwrap_or(defaults.no_tools_guidance),
            fallback_response_keys: r
                .fallback_response_keys
                .clone()
                .unwrap_or(defaults.fallback_response_keys),
        }
    };
    // Persist system_prompt in the extensible custom map (PostcardAppConfig has no dedicated field).
    let mut custom = std::collections::HashMap::new();
    if let Some(sp) = &app.runtime_config.system_prompt {
        custom.insert("system_prompt".to_string(), sp.clone());
    }
    let pc = PostcardAppConfig {
        model: PostcardModelConfig {
            default_provider: app.runtime_config.default_provider.clone(),
            model: app.runtime_config.model.clone(),
        },
        providers: providers_to_postcard(app.providers.clone()),
        prompts: postcard_prompts,
        custom,
        ..Default::default()
    };
    save_app_config(&pc, None).map_err(|error| error.to_string())?;

    let new_client = build_runtime_client(&app.runtime_config, &app.providers)
        .map_err(|error| error.to_string())?;
    app.snapshot = new_client.config_snapshot();
    app.tools = new_client.tools().len();
    *client = new_client;

    Ok(())
}

fn find_provider<'a>(
    providers: &'a [ModelProviderConfig],
    provider_input: &str,
) -> Option<&'a ModelProviderConfig> {
    let needle = provider_input.trim();
    providers
        .iter()
        .find(|provider| provider.id.eq_ignore_ascii_case(needle))
}

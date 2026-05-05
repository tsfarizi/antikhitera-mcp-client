use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use antikythera_core::application::agent::{AgentOutcome, AgentStep};
use antikythera_core::application::client::{ChatResult, McpClient};
use antikythera_core::config::AppConfig;
use antikythera_core::get_latest_logs;
use antikythera_core::infrastructure::model::DynamicModelProvider;
use antikythera_core::{ConfigLogger, ProviderLogger};
use antikythera_sdk::sdk_logging::get_latest_sdk_logs;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::CliResult;
use crate::infrastructure::history::{ChatTurn, TurnRole};
use crate::infrastructure::llm::ModelProviderConfig;
use crate::infrastructure::llm::clear_stream_event_sink;
use crate::runtime::build_runtime_client;

use super::app::ChatApp;
use super::handlers::commands::{apply_runtime_selection, reconfigure_runtime};
use super::handlers::history_handler::handle_history_key;
use super::handlers::settings_handler::handle_settings_key;
use super::handlers::submit::submit_input;
use super::render::draw;
use super::types::{PendingResponse, UiMessage, UiTone};

// TODO: Consider splitting this file further — at ~637 lines it handles event
// loop, key dispatch, message rendering, log fetching, and session management.
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

    // Register builtin MCP server with in-process tool implementations.
    let builtin_transports = {
        use antikythera_core::application::tooling::{
            BuiltinTransport, ServerToolInfo, TaskSupport, ToolAnnotations, ToolExecution,
        };
        use antikythera_core::config::server::{ServerConfig as CoreServerConfig, TransportType};
        use antikythera_core::config::tool::ToolConfig;
        use std::collections::HashMap;

        let builtin_server_name = "builtin_time";
        if !config.servers.iter().any(|s| s.name == builtin_server_name) {
            config.servers.push(CoreServerConfig {
                name: builtin_server_name.to_string(),
                transport: TransportType::Builtin,
                command: None,
                args: Vec::new(),
                env: HashMap::new(),
                workdir: None,
                url: None,
                headers: HashMap::new(),
                default_timezone: None,
                default_city: None,
            });
        }

        let builtin_tools = [("get_current_date", "Get today's date in dd mm yyyy format")];
        for (tool_name, tool_desc) in builtin_tools {
            if !config.tools.iter().any(|t| t.name == tool_name) {
                config.tools.push(ToolConfig {
                    name: tool_name.to_string(),
                    description: Some(tool_desc.to_string()),
                    server: Some(builtin_server_name.to_string()),
                });
            }
        }

        let tool_infos = vec![ServerToolInfo {
            name: "get_current_date".to_string(),
            title: Some("Current Date Provider".to_string()),
            description: Some(
                "Get today's date in dd mm yyyy format. Returns the current date based on the system clock."
                    .to_string(),
            ),
            icons: None,
            input_schema: Some(serde_json::json!({
                "type": "object",
                "additionalProperties": false
            })),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "Today's date in dd mm yyyy format" },
                    "day": { "type": "string" },
                    "month": { "type": "string" },
                    "year": { "type": "string" }
                },
                "required": ["date"]
            })),
            annotations: Some(ToolAnnotations {
                audience: Some(vec!["user".to_string(), "assistant".to_string()]),
                priority: Some(1.0),
                last_modified: None,
            }),
            execution: Some(ToolExecution {
                task_support: Some(TaskSupport::Forbidden),
            }),
        }];

        let transport = BuiltinTransport::with_tools(builtin_server_name, tool_infos).with_handler(
            "get_current_date",
            |_args| {
                let now = chrono::Local::now();
                Ok(serde_json::json!({
                    "date": now.format("%d %m %Y").to_string(),
                    "day": now.format("%A").to_string(),
                    "month": now.format("%B").to_string(),
                    "year": now.format("%Y").to_string(),
                }))
            },
        );

        let mut map = HashMap::new();
        map.insert(builtin_server_name.to_string(), Arc::new(transport));
        map
    };

    // Customise prompts to enforce strict MCP tool format.
    // The template is the first thing the LLM sees — format rules go here.
    config.prompts.template = Some(
        concat!(
            "You are a helpful AI assistant.\n\n",
            // --- Format rules first so the model sees them immediately ---
            "CRITICAL: Your ENTIRE response must be exactly ONE of the two JSON objects below.\n",
            "Do NOT return plain text. Do NOT wrap in markdown. Do NOT add commentary.\n\n",

            "=== TOOL CALL (to use a tool) ===\n",
            r#"{"action":"call_tool","tool":"TOOL_NAME","input":{...}}"#,
            "\n",
            "Example: ",
            r#"{"action":"call_tool","tool":"get_current_date","input":{}}"#,
            "\n",
            r#"WRONG: "tool_call", "tool_calls", "function", "parameters", "arguments" — all rejected."#,
            "\n\n",

            "=== FINAL RESPONSE (to answer the user) ===\n",
            r#"{"action":"final","response":{"content":"your answer here"}}"#,
            "\n",
            "IMPORTANT: The ENTIRE response must be the JSON above. Do NOT return just ",
            r#"{"content":"..."} "#,
            "without the action/response wrapper.\n",
            "Only include a ",
            r#""data" "#,
            "field if you used a tool — reference the step like ",
            r#""data":"step_0""#,
            ". If no tool was used, omit the data field entirely.\n\n",

            "{{custom_instruction}}\n\n",
            "{{language_guidance}}\n\n",
            "{{tool_guidance}}",
        ).to_string(),
    );

    ConfigLogger::new(&antikythera_core::get_active_session()).info(format!(
        "Building runtime client | provider={} model={} providers_count={}",
        config.default_provider,
        config.model,
        providers.len(),
    ));

    let client = build_runtime_client(&config, &providers, builtin_transports.clone())?;
    let snapshot = client.config_snapshot();
    let tools = client.tools().len();
    let mut app = ChatApp::new(config, providers, snapshot, tools, builtin_transports);
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
                // Extract HH:MM:SS from ISO 8601 timestamp.
                let time = entry.timestamp.get(11..19).unwrap_or("--:--:--");
                // Include context payload (FFI args, tool names, etc.) when present.
                if let Some(ctx) = &entry.context {
                    format!("{time} [{level:<5}][{source}] {} | {ctx}", entry.message)
                } else {
                    format!("{time} [{level:<5}][{source}] {}", entry.message)
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

pub(crate) enum KeyAction {
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

fn apply_chat_result(app: &mut ChatApp, result: ChatResult) {
    ProviderLogger::new(&antikythera_core::get_active_session()).info(format!(
        "CORE → CLI: chat response received | provider={} model={} session={} chars={}",
        result.provider,
        result.model,
        result.session_id,
        result.content.len(),
    ));
    app.session_id = Some(result.session_id.clone());
    antikythera_core::set_active_session(&result.session_id);
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
    ProviderLogger::new(&antikythera_core::get_active_session()).info(format!(
        "CORE → CLI: agent outcome received | session={} steps={} chars={}",
        outcome.session_id,
        outcome.steps.len(),
        outcome.response.to_string().len(),
    ));
    app.session_id = Some(outcome.session_id.clone());
    antikythera_core::set_active_session(&outcome.session_id);
    app.status = format!("Agent selesai dengan {} langkah tool.", outcome.steps.len());
    let response_text = format_agent_response(&outcome.response);
    app.push_message(UiMessage::new(
        "Agent",
        response_text.clone(),
        UiTone::Assistant,
    ));
    // Scroll conversation to show the response.
    app.conversation_scroll = scroll_to_bottom(&app.messages, app.conversation_scroll);

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

/// Estimate the scroll offset needed to show the bottom of the conversation.
/// Each message body line counts as 1 visible line; message headers add 1 line each;
/// blank separators add 1 line each.
pub(super) fn scroll_to_bottom(messages: &[UiMessage], _current_scroll: u16) -> u16 {
    let total_lines: usize = messages
        .iter()
        .map(|m| 2 + m.body.lines().count()) // title line + 1 body line minimum + separator
        .sum();
    total_lines.saturating_sub(8) as u16 // ~8 lines of viewport
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

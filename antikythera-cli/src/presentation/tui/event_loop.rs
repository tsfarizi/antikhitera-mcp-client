mod key_dispatch;
mod result_handler;
mod startup;

use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use antikythera_core::ConfigLogger;
use antikythera_core::application::client::McpClient;
use antikythera_core::config::AppConfig;
use antikythera_core::get_latest_logs;
use antikythera_core::infrastructure::model::DynamicModelProvider;
use antikythera_sdk::sdk_logging::get_latest_sdk_logs;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::CliResult;
use crate::infrastructure::llm::ModelProviderConfig;
use crate::infrastructure::llm::clear_stream_event_sink;
use crate::runtime::build_runtime_client;

use key_dispatch::handle_key_event;
use result_handler::{apply_agent_outcome, apply_chat_result};
use startup::bootstrap_servers_and_transports;

pub(crate) use key_dispatch::KeyAction;
pub(crate) use result_handler::scroll_to_bottom;

use super::app::ChatApp;
use super::handlers::commands::{apply_runtime_selection, reconfigure_runtime};
use super::handlers::submit::submit_input;
use super::render::draw;
use super::types::{PendingResponse, UiMessage, UiTone};

pub async fn run_chat_app(
    mut config: AppConfig,
    providers: Vec<ModelProviderConfig>,
) -> CliResult<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Server auto-discovery, builtin transport registration, prompt customisation.
    let (discovery_msg, builtin_transports) = bootstrap_servers_and_transports(&mut config).await;

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

use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use antikythera_core::application::agent::{Agent, AgentOptions, AgentOutcome, AgentStep};
use antikythera_core::application::client::{
    ChatRequest, ChatResult, ClientConfigSnapshot, McpClient,
};
use antikythera_core::config::{AppConfig, PromptsConfig};
use antikythera_core::get_latest_logs;
use antikythera_sdk::sdk_logging::get_latest_sdk_logs;
use antikythera_core::infrastructure::model::DynamicModelProvider;
use crate::config::{save_app_config, AppConfig as PostcardAppConfig, ModelConfig as PostcardModelConfig};
use crate::infrastructure::llm::{ModelInfo, ModelProviderConfig, providers_to_postcard};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use tokio::sync::oneshot;

use crate::CliResult;
use crate::runtime::{build_runtime_client, materialize_runtime_config};

const MAX_VISIBLE_MESSAGES: usize = 12;

/// Result received from a spawned chat or agent task via a oneshot channel.
enum PendingResponse {
    Chat(Result<ChatResult, String>),
    Agent(Result<AgentOutcome, String>),
}

// ── Settings Panel types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Provider = 0,
    Model    = 1,
    Prompts  = 2,
    System   = 3,
    Agent    = 4,
}

impl SettingsTab {
    const COUNT: usize = 5;
    const ALL: [SettingsTab; 5] = [
        SettingsTab::Provider,
        SettingsTab::Model,
        SettingsTab::Prompts,
        SettingsTab::System,
        SettingsTab::Agent,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Provider => "Provider",
            Self::Model    => "Model",
            Self::Prompts  => "Prompts",
            Self::System   => "System Prompt",
            Self::Agent    => "Agent",
        }
    }

    fn next(self) -> Self { Self::ALL[(self as usize + 1) % Self::COUNT] }
    fn prev(self) -> Self { Self::ALL[(self as usize + Self::COUNT - 1) % Self::COUNT] }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptField {
    Template            = 0,
    ToolGuidance        = 1,
    FallbackGuidance    = 2,
    JsonRetryMessage    = 3,
    ToolResultInstr     = 4,
    AgentInstructions   = 5,
    UiInstructions      = 6,
    LanguageInstructions = 7,
    AgentMaxStepsError  = 8,
    NoToolsGuidance     = 9,
}

impl PromptField {
    const COUNT: usize = 10;
    const ALL: [PromptField; 10] = [
        PromptField::Template,
        PromptField::ToolGuidance,
        PromptField::FallbackGuidance,
        PromptField::JsonRetryMessage,
        PromptField::ToolResultInstr,
        PromptField::AgentInstructions,
        PromptField::UiInstructions,
        PromptField::LanguageInstructions,
        PromptField::AgentMaxStepsError,
        PromptField::NoToolsGuidance,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Template             => "Template",
            Self::ToolGuidance         => "Tool Guidance",
            Self::FallbackGuidance     => "Fallback Guidance",
            Self::JsonRetryMessage     => "JSON Retry Msg",
            Self::ToolResultInstr      => "Tool Result Instr",
            Self::AgentInstructions    => "Agent Instructions",
            Self::UiInstructions       => "UI Instructions",
            Self::LanguageInstructions => "Language Instr",
            Self::AgentMaxStepsError   => "Max Steps Error",
            Self::NoToolsGuidance      => "No Tools Guidance",
        }
    }

    fn get_from(self, p: &PromptsConfig) -> String {
        match self {
            Self::Template             => p.template().to_string(),
            Self::ToolGuidance         => p.tool_guidance().to_string(),
            Self::FallbackGuidance     => p.fallback_guidance().to_string(),
            Self::JsonRetryMessage     => p.json_retry_message().to_string(),
            Self::ToolResultInstr      => p.tool_result_instruction().to_string(),
            Self::AgentInstructions    => p.agent_instructions().to_string(),
            Self::UiInstructions       => p.ui_instructions().to_string(),
            Self::LanguageInstructions => p.language_instructions().to_string(),
            Self::AgentMaxStepsError   => p.agent_max_steps_error().to_string(),
            Self::NoToolsGuidance      => p.no_tools_guidance().to_string(),
        }
    }

    fn set_into(self, p: &mut PromptsConfig, value: String) {
        let v = if value.is_empty() { None } else { Some(value) };
        match self {
            Self::Template             => p.template = v,
            Self::ToolGuidance         => p.tool_guidance = v,
            Self::FallbackGuidance     => p.fallback_guidance = v,
            Self::JsonRetryMessage     => p.json_retry_message = v,
            Self::ToolResultInstr      => p.tool_result_instruction = v,
            Self::AgentInstructions    => p.agent_instructions = v,
            Self::UiInstructions       => p.ui_instructions = v,
            Self::LanguageInstructions => p.language_instructions = v,
            Self::AgentMaxStepsError   => p.agent_max_steps_error = v,
            Self::NoToolsGuidance      => p.no_tools_guidance = v,
        }
    }
}

struct SettingsPanel {
    open: bool,
    tab: SettingsTab,
    provider_cursor: usize,
    model_cursor: usize,
    prompt_cursor: usize,
    editing: bool,
    edit_buffer: String,
    pending_provider_idx: usize,
    pending_model_idx: usize,
    pending_system_prompt: String,
    pending_prompts: PromptsConfig,
    pending_agent_mode: bool,
    /// Whether the "add model" input row is active on the Model tab.
    model_add_mode: bool,
    /// Buffer for the new model name being typed on the Model tab.
    model_add_buffer: String,
}

impl SettingsPanel {
    fn new() -> Self {
        Self {
            open: false,
            tab: SettingsTab::Provider,
            provider_cursor: 0,
            model_cursor: 0,
            prompt_cursor: 0,
            editing: false,
            edit_buffer: String::new(),
            pending_provider_idx: 0,
            pending_model_idx: 0,
            pending_system_prompt: String::new(),
            pending_prompts: PromptsConfig::default(),
            pending_agent_mode: true,
            model_add_mode: false,
            model_add_buffer: String::new(),
        }
    }

    fn open_with(
        &mut self,
        app_provider: &str,
        app_model: &str,
        config: &AppConfig,
        providers: &[ModelProviderConfig],
        agent_mode: bool,
    ) {
        self.open = true;
        self.tab = SettingsTab::Provider;
        self.editing = false;
        self.edit_buffer.clear();
        self.pending_system_prompt = config.system_prompt.clone().unwrap_or_default();
        self.pending_prompts = config.prompts.clone();
        self.pending_agent_mode = agent_mode;
        self.pending_provider_idx = providers
            .iter()
            .position(|p| p.id == app_provider)
            .unwrap_or(0);
        self.provider_cursor = self.pending_provider_idx;
        self.pending_model_idx = providers
            .get(self.pending_provider_idx)
            .and_then(|p| p.models.iter().position(|m| m.name == app_model))
            .unwrap_or(0);
        self.model_cursor = self.pending_model_idx;
        self.prompt_cursor = 0;
        self.model_add_mode = false;
        self.model_add_buffer.clear();
    }
}

const SLASH_COMMANDS: [(&str, &str); 10] = [
    ("help", "Tampilkan perintah yang tersedia"),
    ("providers", "Tampilkan provider dan model yang tersedia"),
    ("use", "Pilih provider aktif: /use <provider> [model]"),
    ("model", "Ganti model provider aktif: /model <nama-model>"),
    ("config", "Ringkasan provider, prompt, tools, dan server"),
    ("tools", "Daftar tools aktif pada sesi ini"),
    ("agent", "Toggle atau set mode agent: /agent on|off|toggle"),
    ("reset", "Mulai sesi baru dan hapus riwayat UI"),
    ("clear", "Alias untuk /reset"),
    ("exit", "Tutup TUI"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiTone {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone)]
struct UiMessage {
    title: String,
    body: String,
    tone: UiTone,
}

impl UiMessage {
    fn new(title: impl Into<String>, body: impl Into<String>, tone: UiTone) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            tone,
        }
    }
}

struct ChatApp {
    runtime_config: AppConfig,
    provider: String,
    model: String,
    session_id: Option<String>,
    input: String,
    settings: SettingsPanel,
    agent_mode: bool,
    status: String,
    tools: usize,
    providers: Vec<ModelProviderConfig>,
    snapshot: ClientConfigSnapshot,
    messages: Vec<UiMessage>,
    /// Recent log lines pulled from the core logging system (WASM FFI source).
    log_lines: Vec<String>,
    loading: bool,
    should_quit: bool,
    /// In-flight request receiver. Set when a chat/agent task has been spawned;
    /// cleared when the result arrives or the channel is closed.
    pending_rx: Option<oneshot::Receiver<PendingResponse>>,
}

impl ChatApp {
    fn new(runtime_config: AppConfig, providers: Vec<ModelProviderConfig>, snapshot: ClientConfigSnapshot, tools: usize) -> Self {
        let mut app = Self {
            provider: runtime_config.default_provider.clone(),
            model: runtime_config.model.clone(),
            session_id: None,
            input: String::new(),
            settings: SettingsPanel::new(),
            agent_mode: true,
            status: "Siap. Ketik pesan atau /help. F2 = Settings Panel.".to_string(),
            tools,
            providers,
            runtime_config,
            snapshot,
            messages: Vec::new(),
            log_lines: Vec::new(),
            loading: false,
            should_quit: false,
            pending_rx: None,
        };
        app.messages.push(UiMessage::new(
            "Welcome",
            "Interactive mode siap. Gunakan /use <provider> [model] atau /model <nama-model> untuk mengganti backend langsung dari TUI.",
            UiTone::System,
        ));
        app
    }

    fn push_message(&mut self, message: UiMessage) {
        self.messages.push(message);
        if self.messages.len() > 64 {
            let excess = self.messages.len() - 64;
            self.messages.drain(0..excess);
        }
    }

    fn suggestions(&self) -> Vec<(&'static str, &'static str)> {
        slash_command_suggestions(&self.input)
    }

    fn reset_session(&mut self) {
        self.session_id = None;
        self.status =
            "Sesi direset. Riwayat host baru akan dimulai pada pesan berikutnya.".to_string();
        self.push_message(UiMessage::new(
            "Session Reset",
            "Riwayat sesi UI dibersihkan. Context baru akan dibuat saat Anda mengirim pesan berikutnya.",
            UiTone::System,
        ));
    }
}

pub async fn run_chat_app(config: AppConfig, providers: Vec<ModelProviderConfig>) -> CliResult<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let client = build_runtime_client(&config, &providers)?;
    let snapshot = client.config_snapshot();
    let tools = client.tools().len();
    let result = run_loop(&mut terminal, client, ChatApp::new(config, providers, snapshot, tools)).await;

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
        // Poll for a completed in-flight request spawned in a previous iteration.
        if let Some(mut rx) = app.pending_rx.take() {
            use tokio::sync::oneshot::error::TryRecvError;
            match rx.try_recv() {
                Ok(PendingResponse::Chat(Ok(result))) => {
                    app.loading = false;
                    apply_chat_result(&mut app, result);
                }
                Ok(PendingResponse::Chat(Err(msg))) => {
                    app.loading = false;
                    app.status = "Model gagal menjawab.".to_string();
                    app.push_message(UiMessage::new("Model Error", msg, UiTone::Error));
                }
                Ok(PendingResponse::Agent(Ok(outcome))) => {
                    app.loading = false;
                    apply_agent_outcome(&mut app, outcome);
                }
                Ok(PendingResponse::Agent(Err(msg))) => {
                    app.loading = false;
                    app.status = "Agent gagal menyelesaikan permintaan.".to_string();
                    app.push_message(UiMessage::new("Agent Error", msg, UiTone::Error));
                }
                Err(TryRecvError::Empty) => {
                    // Task still running — put the receiver back.
                    app.pending_rx = Some(rx);
                }
                Err(TryRecvError::Closed) => {
                    app.loading = false;
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

    // F2 opens the full settings panel.
    if key.code == KeyCode::F(2) {
        let provider = app.provider.clone();
        let model = app.model.clone();
        let agent_mode = app.agent_mode;
        let config = app.runtime_config.clone();
        let providers = app.providers.clone();
        app.settings.open_with(&provider, &model, &config, &providers, agent_mode);
        app.status =
            "Settings terbuka. Tab/BackTab=ganti tab | ↑↓=navigasi | Enter=pilih | Ctrl+S=simpan | Esc=tutup".to_string();
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
                    && let Some(provider) =
                        app.providers.get_mut(app.settings.pending_provider_idx)
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
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Only allow printable ASCII (no whitespace except hyphen/dot).
                if ch.is_alphanumeric() || matches!(ch, '-' | '.' | '_' | ':') {
                    app.settings.model_add_buffer.push(ch);
                }
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
            SettingsTab::Prompts => {
                if app.settings.prompt_cursor + 1 < PromptField::COUNT {
                    app.settings.prompt_cursor += 1;
                }
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
                app.settings.pending_model_idx =
                    app.settings.pending_model_idx.min(new_len.saturating_sub(1));
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

    let (tx, rx) = oneshot::channel();
    app.pending_rx = Some(rx);

    if app.agent_mode {
        let options = AgentOptions {
            session_id: app.session_id.clone(),
            ..AgentOptions::default()
        };
        let client_arc = Arc::clone(client);
        tokio::spawn(async move {
            let result = Agent::new(client_arc)
                .run(input, options)
                .await
                .map_err(|e| e.user_message());
            let _ = tx.send(PendingResponse::Agent(result));
        });
    } else {
        let client_arc = Arc::clone(client);
        let session_id = app.session_id.clone();
        tokio::spawn(async move {
            let result = client_arc
                .chat(ChatRequest {
                    prompt: input,
                    attachments: Vec::new(),
                    system_prompt: None,
                    session_id,
                    raw_mode: false,
                    bypass_template: false,
                    force_json: false,
                })
                .await
                .map_err(|e| e.user_message());
            let _ = tx.send(PendingResponse::Chat(result));
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
        result.content,
        UiTone::Assistant,
    ));
}

fn apply_agent_outcome(app: &mut ChatApp, outcome: AgentOutcome) {
    app.session_id = Some(outcome.session_id.clone());
    app.status = format!("Agent selesai dengan {} langkah tool.", outcome.steps.len());
    app.push_message(UiMessage::new(
        "Agent",
        format_agent_response(&outcome.response),
        UiTone::Assistant,
    ));

    if !outcome.steps.is_empty() {
        app.push_message(UiMessage::new(
            "Tool Trace",
            render_steps_summary(&outcome.steps),
            UiTone::System,
        ));
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

fn draw(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
    // ── Vertical skeleton ───────────────────────────────────────────────────
    //  [0] header (3 rows)
    //  [1] content area (min 16 rows)
    //  [2] prompt / model-edit bar (3 rows)
    //  [3] footer / status (2 rows)
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(16),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    // ── Horizontal split: chat | right panel ───────────────────────────────
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(layout[1]);

    // Right panel: context (top 40%) + WASM/FFI log (bottom 60%)
    let right_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(content[1]);

    // ── Header ──────────────────────────────────────────────────────────────
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " Antikythera CLI ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} / {}", app.provider, app.model),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            if app.agent_mode { "Agent Loop" } else { "Direct Chat" },
            Style::default().fg(Color::Yellow),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Session"));
    frame.render_widget(header, layout[0]);

    // ── Conversation ────────────────────────────────────────────────────────
    let messages = app
        .messages
        .iter()
        .rev()
        .take(MAX_VISIBLE_MESSAGES)
        .collect::<Vec<_>>();
    let conversation = Paragraph::new(render_messages(messages.into_iter().rev()))
        .block(Block::default().borders(Borders::ALL).title("Conversation"))
        .wrap(Wrap { trim: false });
    frame.render_widget(conversation, content[0]);

    // ── Context sidebar ─────────────────────────────────────────────────────
    let sidebar_items = build_sidebar_items(app)
        .into_iter()
        .map(ListItem::new)
        .collect::<Vec<_>>();
    let sidebar =
        List::new(sidebar_items).block(Block::default().borders(Borders::ALL).title("Context"));
    frame.render_widget(sidebar, right_panel[0]);

    // ── WASM / FFI log panel ────────────────────────────────────────────────
    // Merges core logs (transport, provider, agent) and SDK logs (FFI, WASM).
    // Error entries are surfaced in the chat area; omit them here.
    let log_items: Vec<ListItem<'_>> = app
        .log_lines
        .iter()
        .filter(|line| !line.contains("[ERROR]") && !line.contains("[Error]"))
        .map(|line| {
            let style = if line.contains("[WARN]") || line.contains("[Warn]") {
                Style::default().fg(Color::Yellow)
            } else if line.contains("[DEBUG]") || line.contains("[Debug]") {
                // FFI/SDK calls (sdk: prefix) rendered in Magenta.
                if line.contains("][sdk:") {
                    Style::default().fg(Color::Magenta)
                } else if line.contains("][transport]") {
                    Style::default().fg(Color::Cyan)
                } else if line.contains("][provider]") {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            } else {
                // INFO level: color by source.
                if line.contains("][sdk:") {
                    Style::default().fg(Color::LightMagenta)
                } else if line.contains("][transport]") {
                    Style::default().fg(Color::LightCyan)
                } else if line.contains("][provider]") {
                    Style::default().fg(Color::LightBlue)
                } else if line.contains("][agent]") {
                    Style::default().fg(Color::LightGreen)
                } else {
                    Style::default().fg(Color::Gray)
                }
            };
            ListItem::new(Span::styled(line.clone(), style))
        })
        .collect();
    let log_panel = List::new(log_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("WASM / FFI Logs  [magenta=FFI | cyan=transport | blue=provider | green=agent]")
            .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
    );
    frame.render_widget(log_panel, right_panel[1]);

    // ── Prompt bar ───────────────────────────────────────────────────────────
    {
        let prompt_title = if app.loading {
            "Prompt  [mengirim...]"
        } else {
            "Prompt  [F2 = Settings | Enter = kirim | /help = commands]"
        };
        let input_widget = Paragraph::new(app.input.as_str())
            .block(Block::default().borders(Borders::ALL).title(prompt_title))
            .style(if app.loading {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });
        frame.render_widget(input_widget, layout[2]);
    }

    // ── Command autocomplete overlay ────────────────────────────────────────
    if app.input.starts_with('/') {
        let suggestions = app
            .suggestions()
            .into_iter()
            .map(|(name, description)| ListItem::new(format!("/{name:<10} {description}")))
            .collect::<Vec<_>>();
        let area = centered_rect(72, 34, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(
            List::new(suggestions).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Command Suggestions"),
            ),
            area,
        );
    }

    // ── Footer / status ──────────────────────────────────────────────────────
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" autocomplete  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" submit  "),
        Span::styled("F2", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::raw(" settings  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" quit  "),
        Span::styled(app.status.as_str(), Style::default().fg(Color::Gray)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(footer, layout[3]);

    // ── Settings overlay (drawn on top of everything else) ───────────────────
    if app.settings.open {
        draw_settings_overlay(frame, app);
    }
}

fn render_messages<'a>(messages: impl Iterator<Item = &'a UiMessage>) -> Text<'static> {
    let mut lines = Vec::new();
    for message in messages {
        let tone_style = match message.tone {
            UiTone::User => Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
            UiTone::Assistant => Style::default()
                .fg(Color::Black)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            UiTone::System => Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            UiTone::Error => Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", message.title), tone_style),
            Span::raw(" "),
        ]));
        for body_line in message.body.lines() {
            lines.push(Line::from(Span::raw(body_line.to_string())));
        }
        lines.push(Line::default());
    }
    Text::from(lines)
}

fn build_sidebar_items(app: &ChatApp) -> Vec<String> {
    let session = app.session_id.as_deref().unwrap_or("belum ada");
    let provider_lines = app
        .providers
        .iter()
        .map(|provider| {
            let marker = if provider.id == app.provider {
                "*"
            } else {
                " "
            };
            let models = provider
                .models
                .iter()
                .map(|model| {
                    model
                        .display_name
                        .clone()
                        .unwrap_or_else(|| model.name.clone())
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{marker} {} [{}]\n  {}",
                provider.id, provider.provider_type, models
            )
        })
        .collect::<Vec<_>>();

    vec![
        format!("Provider aktif : {}", app.provider),
        format!("Model aktif    : {}", app.model),
        format!(
            "Mode           : {}",
            if app.agent_mode { "agent" } else { "chat" }
        ),
        format!("Tools aktif    : {}", app.tools),
        format!("Session        : {}", session),
        String::new(),
        "Providers".to_string(),
        provider_lines.join("\n"),
    ]
}

fn centered_rect(
    horizontal_percent: u16,
    vertical_percent: u16,
    area: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - vertical_percent) / 2),
            Constraint::Percentage(vertical_percent),
            Constraint::Percentage((100 - vertical_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - horizontal_percent) / 2),
            Constraint::Percentage(horizontal_percent),
            Constraint::Percentage((100 - horizontal_percent) / 2),
        ])
        .split(vertical[1])[1]
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

fn render_config_snapshot(snapshot: &ClientConfigSnapshot) -> String {
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
    let pc = PostcardAppConfig {
        model: PostcardModelConfig {
            default_provider: app.runtime_config.default_provider.clone(),
            model: app.runtime_config.model.clone(),
        },
        providers: providers_to_postcard(app.providers.clone()),
        ..Default::default()
    };
    save_app_config(&pc, None).map_err(|error| error.to_string())?;

    let new_client =
        build_runtime_client(&app.runtime_config, &app.providers).map_err(|error| error.to_string())?;
    app.snapshot = new_client.config_snapshot();
    app.tools = new_client.tools().len();
    *client = new_client;

    Ok(())
}

// ── Settings overlay draw functions ──────────────────────────────────────────

fn draw_settings_overlay(frame: &mut ratatui::Frame<'_>, app: &ChatApp) {
    let area = frame.area();
    // Blank the entire terminal before drawing the overlay.
    frame.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(" ⚙  Settings  [Tab/BackTab=ganti tab | ↑↓=nav | Enter=pilih | Ctrl+S=simpan | Esc=tutup] ")
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .split(inner);

    // Tab bar — highlight the active tab.
    let mut tab_spans: Vec<Span> = Vec::new();
    for (i, tab) in SettingsTab::ALL.iter().enumerate() {
        let label = format!(" [{}] {} ", i + 1, tab.label());
        if *tab == app.settings.tab {
            tab_spans.push(Span::styled(
                label,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            tab_spans.push(Span::styled(label, Style::default().fg(Color::Cyan)));
        }
        tab_spans.push(Span::raw("  "));
    }
    let tab_bar =
        Paragraph::new(Line::from(tab_spans)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(tab_bar, layout[0]);

    // Render the content area for the active tab.
    match app.settings.tab {
        SettingsTab::Provider => draw_settings_tab_provider(frame, app, layout[1]),
        SettingsTab::Model    => draw_settings_tab_model(frame, app, layout[1]),
        SettingsTab::Prompts  => draw_settings_tab_prompts(frame, app, layout[1]),
        SettingsTab::System   => draw_settings_tab_system(frame, app, layout[1]),
        SettingsTab::Agent    => draw_settings_tab_agent(frame, app, layout[1]),
    }
}

fn draw_settings_tab_provider(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let items: Vec<ListItem> = app
        .providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let selected = i == app.settings.pending_provider_idx;
            let cursor = i == app.settings.provider_cursor;
            let radio = if selected { "◉" } else { "○" };
            let arrow = if cursor { "▶" } else { " " };
            let style = if cursor {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            ListItem::new(format!("{arrow} {radio} {:<18} [{}]", p.id, p.provider_type))
                .style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Provider  [↑↓=navigasi | Enter=pilih & ke tab Model]")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, cols[0]);

    if let Some(p) = app.providers.get(app.settings.provider_cursor) {
        let models_text = p
            .models
            .iter()
            .map(|m| format!("  • {}", m.display_name.as_deref().unwrap_or(&m.name)))
            .collect::<Vec<_>>()
            .join("\n");
        let api_status = if p.api_key.is_some() {
            "✓ tersedia"
        } else {
            "✗ tidak ada (pakai env var)"
        };
        let detail = format!(
            "ID       : {}\nType     : {}\nEndpoint : {}\nAPI Key  : {}\n\nModels:\n{}",
            p.id,
            p.provider_type,
            p.endpoint,
            api_status,
            if models_text.is_empty() {
                "  (tidak ada model terdaftar)".to_string()
            } else {
                models_text
            }
        );
        let widget = Paragraph::new(detail)
            .block(Block::default().borders(Borders::ALL).title("Detail Provider"))
            .wrap(Wrap { trim: false });
        frame.render_widget(widget, cols[1]);
    }
}

fn draw_settings_tab_model(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    // Reserve the bottom row for the add-model input bar when active.
    let (list_area, input_area) = if app.settings.model_add_mode {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(list_area);

    let Some(provider) = app.providers.get(app.settings.pending_provider_idx) else {
        let msg = Paragraph::new("Pilih provider terlebih dahulu di tab [1] Provider.")
            .block(Block::default().borders(Borders::ALL).title("Model"));
        frame.render_widget(msg, area);
        return;
    };

    let items: Vec<ListItem> = provider
        .models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let selected = i == app.settings.pending_model_idx;
            let cursor = i == app.settings.model_cursor;
            let radio = if selected { "◉" } else { "○" };
            let arrow = if cursor { "▶" } else { " " };
            let style = if cursor {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            let name = m.display_name.as_deref().unwrap_or(&m.name);
            ListItem::new(format!("{arrow} {radio} {name}")).style(style)
        })
        .collect();

    let list_title = format!(
        "Model '{}' [↑↓=navigasi | Enter=pilih | a=tambah | d=hapus]",
        provider.id
    );
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(list_title)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, cols[0]);

    // Right column: detail or empty state hint.
    if provider.models.is_empty() {
        let hint = Paragraph::new(
            "Belum ada model.\n\nTekan [a] untuk menambahkan nama model\n(contoh: gemini-2.0-flash)",
        )
        .block(Block::default().borders(Borders::ALL).title("Detail Model"))
        .wrap(Wrap { trim: false });
        frame.render_widget(hint, cols[1]);
    } else if let Some(m) = provider.models.get(app.settings.model_cursor) {
        let status = if app.settings.model_cursor == app.settings.pending_model_idx {
            "◉ terpilih"
        } else {
            "○ belum dipilih (tekan Enter untuk memilih)"
        };
        let detail = format!(
            "Name         : {}\nDisplay Name : {}\n\nStatus       : {}",
            m.name,
            m.display_name.as_deref().unwrap_or("(sama dengan name)"),
            status
        );
        let widget = Paragraph::new(detail)
            .block(Block::default().borders(Borders::ALL).title("Detail Model"))
            .wrap(Wrap { trim: false });
        frame.render_widget(widget, cols[1]);
    }

    // Add-model input bar at the bottom.
    if let Some(input_rect) = input_area {
        let prompt = format!("Nama model: {}█", app.settings.model_add_buffer);
        let input_widget = Paragraph::new(prompt)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Tambah Model  [Enter=simpan | Esc=batal]")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(input_widget, input_rect);
    }
}

fn draw_settings_tab_prompts(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let items: Vec<ListItem> = PromptField::ALL
        .iter()
        .enumerate()
        .map(|(i, &field)| {
            let cursor = i == app.settings.prompt_cursor;
            let arrow = if cursor { "▶" } else { " " };
            let preview = field
                .get_from(&app.settings.pending_prompts)
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(58)
                .collect::<String>();
            let style = if cursor {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{arrow} {:<22} {}", field.label(), preview)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Prompt Fields  [↑↓=pilih | Enter=edit | Ctrl+Enter=simpan field | Esc=batal edit]")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, rows[0]);

    if app.settings.editing {
        let edit_widget = Paragraph::new(app.settings.edit_buffer.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Edit Field  [Ctrl+Enter=simpan | Esc=batal | Enter=baris baru]")
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(edit_widget, rows[1]);
    } else if let Some(&field) = PromptField::ALL.get(app.settings.prompt_cursor) {
        let preview_text = field.get_from(&app.settings.pending_prompts);
        let preview_widget = Paragraph::new(preview_text.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Preview: {}  [Enter=edit]", field.label()))
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(preview_widget, rows[1]);
    }
}

fn draw_settings_tab_system(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(8)])
        .split(area);

    let info = Paragraph::new(
        "System prompt di-inject ke setiap sesi baru sebagai instruksi dasar.\n\
         Biarkan kosong untuk menggunakan template default dari PromptsConfig.\n\
         Tekan Enter untuk mulai edit. Ctrl+Enter untuk simpan perubahan.",
    )
    .block(Block::default().borders(Borders::ALL).title("Info"))
    .wrap(Wrap { trim: false });
    frame.render_widget(info, rows[0]);

    if app.settings.editing {
        let edit = Paragraph::new(app.settings.edit_buffer.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Edit System Prompt  [Ctrl+Enter=simpan | Esc=batal | Enter=baris baru]")
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(edit, rows[1]);
    } else {
        let current = if app.settings.pending_system_prompt.is_empty() {
            "(kosong — menggunakan template default dari PromptsConfig)".to_string()
        } else {
            app.settings.pending_system_prompt.clone()
        };
        let preview = Paragraph::new(current.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("System Prompt Aktif  [Enter=edit]")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(preview, rows[1]);
    }
}

fn draw_settings_tab_agent(frame: &mut ratatui::Frame<'_>, app: &ChatApp, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(4)])
        .split(area);

    let mode_label = if app.settings.pending_agent_mode {
        "◉ Agent Loop  (aktif)"
    } else {
        "○ Direct Chat (aktif)"
    };
    let content = format!(
        "Mode Eksekusi  : {}\n\n\
         Tekan Enter untuk toggle mode.\n\n\
         ◉ Agent Loop   — Prompt dieksekusi melalui planning loop & tool calls.\n\
         ○ Direct Chat  — Prompt langsung dikirim ke model tanpa loop agent.\n\n\
         Ctrl+S untuk menyimpan semua perubahan settings.",
        mode_label
    );
    let widget = Paragraph::new(content.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Agent Settings  [Enter=toggle | Ctrl+S=simpan semua]")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, rows[0]);

    // Quick summary of all pending changes.
    let provider_name = app
        .providers
        .get(app.settings.pending_provider_idx)
        .map(|p| p.id.as_str())
        .unwrap_or("(tidak ada)");
    let model_name = app
        .providers
        .get(app.settings.pending_provider_idx)
        .and_then(|p| p.models.get(app.settings.pending_model_idx))
        .map(|m| m.name.as_str())
        .unwrap_or("(tidak ada)");
    let summary = format!(
        "Pending Changes:\n  Provider     : {}\n  Model        : {}\n  Mode         : {}\n  System Prompt: {} karakter",
        provider_name,
        model_name,
        if app.settings.pending_agent_mode { "Agent Loop" } else { "Direct Chat" },
        app.settings.pending_system_prompt.len(),
    );
    let summary_widget = Paragraph::new(summary.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Ringkasan Perubahan Pending  [Ctrl+S=terapkan semua]")
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(summary_widget, rows[1]);
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

fn slash_command_suggestions(input: &str) -> Vec<(&'static str, &'static str)> {
    if !input.starts_with('/') {
        return Vec::new();
    }

    let needle = input.trim_start_matches('/').trim().to_ascii_lowercase();
    SLASH_COMMANDS
        .iter()
        .copied()
        .filter(|(command, _)| needle.is_empty() || command.starts_with(&needle))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::ModelInfo;

    fn provider(id: &str, provider_type: &str, model: &str) -> ModelProviderConfig {
        ModelProviderConfig {
            id: id.to_string(),
            provider_type: provider_type.to_string(),
            endpoint: format!("https://{}.example.test", id),
            api_key: None,
            api_path: None,
            models: vec![ModelInfo {
                name: model.to_string(),
                display_name: Some(model.to_string()),
            }],
        }
    }

    #[test]
    fn slash_command_suggestions_match_prefix() {
        let suggestions = slash_command_suggestions("/pr");
        assert!(suggestions.iter().any(|(name, _)| *name == "providers"));
        assert!(suggestions.iter().all(|(name, _)| name.starts_with("pr")));
    }

    #[test]
    fn render_provider_catalog_marks_active_model() {
        let rendered = render_provider_catalog(
            &[provider("gemini", "gemini", "gemini-2.0-flash")],
            "gemini",
            "gemini-2.0-flash",
        );
        assert!(rendered.contains("(aktif)"));
        assert!(rendered.contains("gemini-2.0-flash"));
    }

    #[test]
    fn resolve_provider_selection_uses_provider_default_model_when_missing() {
        let selection = resolve_provider_selection(
            &[
                provider("ollama", "ollama", "llama3.2"),
                provider("openai", "openai", "gpt-4o-mini"),
            ],
            "ollama",
            "llama3.2",
            "openai",
            None,
        )
        .expect("selection should resolve");

        assert_eq!(selection.0, "openai");
        assert_eq!(selection.1, "gpt-4o-mini");
    }

    #[test]
    fn resolve_provider_selection_accepts_explicit_custom_model() {
        let selection = resolve_provider_selection(
            &[provider("gemini", "gemini", "gemini-2.0-flash")],
            "gemini",
            "gemini-2.0-flash",
            "gemini",
            Some("gemini-2.5-pro"),
        )
        .expect("selection should resolve");

        assert_eq!(selection.0, "gemini");
        assert_eq!(selection.1, "gemini-2.5-pro");
    }
}

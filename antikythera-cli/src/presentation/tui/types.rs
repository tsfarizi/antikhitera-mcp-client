use antikythera_core::application::agent::AgentOutcome;
use antikythera_core::application::client::ChatResult;
use antikythera_core::config::{AppConfig, PromptsConfig};

use crate::infrastructure::history::ChatHistorySession;
use crate::infrastructure::llm::ModelProviderConfig;

/// Result received from a spawned chat or agent task via a oneshot channel.
pub(super) enum PendingResponse {
    Chat(Result<ChatResult, String>),
    Agent(Result<AgentOutcome, String>),
}

// ── Settings Panel types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsTab {
    Provider = 0,
    Model = 1,
    Prompts = 2,
    System = 3,
    Agent = 4,
}

impl SettingsTab {
    pub(super) const COUNT: usize = 5;
    pub(super) const ALL: [SettingsTab; 5] = [
        SettingsTab::Provider,
        SettingsTab::Model,
        SettingsTab::Prompts,
        SettingsTab::System,
        SettingsTab::Agent,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Provider => "Provider",
            Self::Model => "Model",
            Self::Prompts => "Prompts",
            Self::System => "System Prompt",
            Self::Agent => "Agent",
        }
    }

    pub(super) fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::COUNT]
    }
    pub(super) fn prev(self) -> Self {
        Self::ALL[(self as usize + Self::COUNT - 1) % Self::COUNT]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PromptField {
    Template = 0,
    ToolGuidance = 1,
    FallbackGuidance = 2,
    JsonRetryMessage = 3,
    ToolResultInstr = 4,
    AgentInstructions = 5,
    UiInstructions = 6,
    LanguageInstructions = 7,
    AgentMaxStepsError = 8,
    NoToolsGuidance = 9,
}

impl PromptField {
    pub(super) const COUNT: usize = 10;
    pub(super) const ALL: [PromptField; 10] = [
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

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Template => "Template",
            Self::ToolGuidance => "Tool Guidance",
            Self::FallbackGuidance => "Fallback Guidance",
            Self::JsonRetryMessage => "JSON Retry Msg",
            Self::ToolResultInstr => "Tool Result Instr",
            Self::AgentInstructions => "Agent Instructions",
            Self::UiInstructions => "UI Instructions",
            Self::LanguageInstructions => "Language Instr",
            Self::AgentMaxStepsError => "Max Steps Error",
            Self::NoToolsGuidance => "No Tools Guidance",
        }
    }

    pub(super) fn get_from(self, p: &PromptsConfig) -> String {
        match self {
            Self::Template => p.template().to_string(),
            Self::ToolGuidance => p.tool_guidance().to_string(),
            Self::FallbackGuidance => p.fallback_guidance().to_string(),
            Self::JsonRetryMessage => p.json_retry_message().to_string(),
            Self::ToolResultInstr => p.tool_result_instruction().to_string(),
            Self::AgentInstructions => p.agent_instructions().to_string(),
            Self::UiInstructions => p.ui_instructions().to_string(),
            Self::LanguageInstructions => p.language_instructions().to_string(),
            Self::AgentMaxStepsError => p.agent_max_steps_error().to_string(),
            Self::NoToolsGuidance => p.no_tools_guidance().to_string(),
        }
    }

    pub(super) fn set_into(self, p: &mut PromptsConfig, value: String) {
        let v = if value.is_empty() { None } else { Some(value) };
        match self {
            Self::Template => p.template = v,
            Self::ToolGuidance => p.tool_guidance = v,
            Self::FallbackGuidance => p.fallback_guidance = v,
            Self::JsonRetryMessage => p.json_retry_message = v,
            Self::ToolResultInstr => p.tool_result_instruction = v,
            Self::AgentInstructions => p.agent_instructions = v,
            Self::UiInstructions => p.ui_instructions = v,
            Self::LanguageInstructions => p.language_instructions = v,
            Self::AgentMaxStepsError => p.agent_max_steps_error = v,
            Self::NoToolsGuidance => p.no_tools_guidance = v,
        }
    }
}

// ── History Browser ─────────────────────────────────────────────────────────

/// Overlay for browsing and managing saved debug chat sessions.
pub(super) struct HistoryBrowser {
    pub(super) open: bool,
    /// Cursor position in the session list.
    pub(super) cursor: usize,
    /// Full turns of the session currently in detail view (`None` = list view).
    pub(super) detail: Option<ChatHistorySession>,
    /// Scroll offset (turn index) inside the detail view.
    pub(super) detail_scroll: usize,
    /// Whether the rename input row is active.
    pub(super) rename_mode: bool,
    /// Text being typed for the rename operation.
    pub(super) rename_buffer: String,
    /// Cached session list — refreshed when the browser is opened.
    pub(super) sessions: Vec<ChatHistorySession>,
}

impl HistoryBrowser {
    pub(super) fn new() -> Self {
        Self {
            open: false,
            cursor: 0,
            detail: None,
            detail_scroll: 0,
            rename_mode: false,
            rename_buffer: String::new(),
            sessions: Vec::new(),
        }
    }

    /// Open the overlay with a pre-loaded session list (avoids double-borrow
    /// when caller needs to split `app.history_store` from `app.history`).
    pub(super) fn open_and_refresh_with(&mut self, sessions: Vec<ChatHistorySession>) {
        self.sessions = sessions;
        self.cursor = 0;
        self.detail = None;
        self.detail_scroll = 0;
        self.rename_mode = false;
        self.rename_buffer.clear();
        self.open = true;
    }
}

pub(super) struct SettingsPanel {
    pub(super) open: bool,
    pub(super) tab: SettingsTab,
    pub(super) provider_cursor: usize,
    pub(super) model_cursor: usize,
    pub(super) prompt_cursor: usize,
    pub(super) editing: bool,
    pub(super) edit_buffer: String,
    pub(super) pending_provider_idx: usize,
    pub(super) pending_model_idx: usize,
    pub(super) pending_system_prompt: String,
    pub(super) pending_prompts: PromptsConfig,
    pub(super) pending_agent_mode: bool,
    /// Whether the "add model" input row is active on the Model tab.
    pub(super) model_add_mode: bool,
    /// Buffer for the new model name being typed on the Model tab.
    pub(super) model_add_buffer: String,
}

impl SettingsPanel {
    pub(super) fn new() -> Self {
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

    pub(super) fn open_with(
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

pub(super) const SLASH_COMMANDS: [(&str, &str); 11] = [
    ("help", "Tampilkan perintah yang tersedia"),
    ("providers", "Tampilkan provider dan model yang tersedia"),
    ("use", "Pilih provider aktif: /use <provider> [model]"),
    ("model", "Ganti model provider aktif: /model <nama-model>"),
    ("config", "Ringkasan provider, prompt, tools, dan server"),
    ("tools", "Daftar tools aktif pada sesi ini"),
    ("agent", "Toggle atau set mode agent: /agent on|off|toggle"),
    ("reset", "Mulai sesi baru dan hapus riwayat UI"),
    ("clear", "Alias untuk /reset"),
    ("history", "Buka browser riwayat sesi chat (F3)"),
    ("exit", "Tutup TUI"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UiTone {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone)]
pub(super) struct UiMessage {
    pub(super) title: String,
    pub(super) body: String,
    pub(super) tone: UiTone,
}

impl UiMessage {
    pub(super) fn new(title: impl Into<String>, body: impl Into<String>, tone: UiTone) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            tone,
        }
    }
}

pub(super) fn slash_command_suggestions(input: &str) -> Vec<(&'static str, &'static str)> {
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

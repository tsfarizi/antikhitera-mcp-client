use std::sync::{Arc, Mutex};

use antikythera_core::application::client::ClientConfigSnapshot;
use antikythera_core::application::resilience::HealthTracker;
use antikythera_core::config::AppConfig;
use tokio::sync::{mpsc, oneshot};

use crate::infrastructure::history::{ChatHistorySession, ChatHistoryStore};
use crate::infrastructure::llm::ModelProviderConfig;

use super::types::{
    HistoryBrowser, PendingResponse, SettingsPanel, UiMessage, UiTone, slash_command_suggestions,
};

pub(super) struct ChatApp {
    pub(super) runtime_config: AppConfig,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) session_id: Option<String>,
    pub(super) input: String,
    pub(super) settings: SettingsPanel,
    pub(super) agent_mode: bool,
    pub(super) status: String,
    pub(super) tools: usize,
    pub(super) providers: Vec<ModelProviderConfig>,
    pub(super) snapshot: ClientConfigSnapshot,
    pub(super) messages: Vec<UiMessage>,
    /// Recent log lines pulled from the core logging system (WASM FFI source).
    pub(super) log_lines: Vec<String>,
    pub(super) loading: bool,
    pub(super) should_quit: bool,
    /// In-flight request receiver. Set when a chat/agent task has been spawned;
    /// cleared when the result arrives or the channel is closed.
    pub(super) pending_rx: Option<oneshot::Receiver<PendingResponse>>,
    // ── Debug history ────────────────────────────────────────────────────────
    /// Persistent store for debug chat history JSON files.
    pub(super) history_store: ChatHistoryStore,
    /// The history session that is currently being built (in-flight).
    pub(super) current_history_session: Option<ChatHistorySession>,
    /// Overlay for browsing and managing saved debug sessions.
    pub(super) history: HistoryBrowser,
    // ── Live streaming ───────────────────────────────────────────────────────
    /// Tokens received so far from the in-flight streaming request.
    pub(super) streaming_content: String,
    /// Channel receiver for streaming token chunks from the background task.
    pub(super) stream_rx: Option<mpsc::UnboundedReceiver<String>>,
    // ── Provider health ──────────────────────────────────────────────────────
    /// Aggregated health metrics for the active LLM provider.
    pub(super) health: Arc<Mutex<HealthTracker>>,
}

impl ChatApp {
    pub(super) fn new(
        runtime_config: AppConfig,
        providers: Vec<ModelProviderConfig>,
        snapshot: ClientConfigSnapshot,
        tools: usize,
    ) -> Self {
        let mut app = Self {
            provider: runtime_config.default_provider.clone(),
            model: runtime_config.model.clone(),
            session_id: None,
            input: String::new(),
            settings: SettingsPanel::new(),
            agent_mode: true,
            status: "Siap. Ketik pesan atau /help. F2 = Settings | F3 = Riwayat.".to_string(),
            tools,
            providers,
            runtime_config,
            snapshot,
            messages: Vec::new(),
            log_lines: Vec::new(),
            loading: false,
            should_quit: false,
            pending_rx: None,
            history_store: ChatHistoryStore::new(),
            current_history_session: None,
            history: HistoryBrowser::new(),
            streaming_content: String::new(),
            stream_rx: None,
            health: Arc::new(Mutex::new(HealthTracker::new())),
        };
        app.messages.push(UiMessage::new(
            "Welcome",
            "Interactive mode siap. Gunakan /use <provider> [model] atau /model <nama-model> untuk mengganti backend langsung dari TUI.",
            UiTone::System,
        ));
        app
    }

    pub(super) fn push_message(&mut self, message: UiMessage) {
        self.messages.push(message);
        if self.messages.len() > 64 {
            let excess = self.messages.len() - 64;
            self.messages.drain(0..excess);
        }
    }

    pub(super) fn suggestions(&self) -> Vec<(&'static str, &'static str)> {
        slash_command_suggestions(&self.input)
    }

    pub(super) fn reset_session(&mut self) {
        self.session_id = None;
        // Finalise the in-flight history session — it was already saved on the
        // last assistant turn, so we just drop the in-memory reference.
        self.current_history_session = None;
        self.status =
            "Sesi direset. Riwayat host baru akan dimulai pada pesan berikutnya.".to_string();
        self.push_message(UiMessage::new(
            "Session Reset",
            "Riwayat sesi UI dibersihkan. Context baru akan dibuat saat Anda mengirim pesan berikutnya.",
            UiTone::System,
        ));
    }
}

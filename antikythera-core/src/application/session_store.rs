use std::collections::VecDeque;

use crate::domain::types::ChatMessage;
use crate::domain::types::MessagePart;
use crate::domain::types::MessageRole;
use crate::logging::SessionLogger;
use antikythera_session::{
    Message, MessagePart as SessionMessagePart, MessageRole as SessionMessageRole, SessionManager,
};

/// Default maximum number of concurrent sessions kept in memory.
///
/// When this limit is reached the least-recently-used session is evicted
/// before a new one is created. This prevents unbounded memory growth in
/// long-running deployments with many ephemeral sessions.
pub(super) const DEFAULT_MAX_SESSIONS: usize = 256;

/// In-memory session store with LRU eviction.
pub(super) struct SessionStore {
    manager: SessionManager,
    /// Access order: front = least recently used, back = most recently used.
    pub(super) order: VecDeque<String>,
    /// Maximum number of sessions to retain simultaneously.
    pub(super) max_sessions: usize,
}

impl SessionStore {
    pub(super) fn new(max_sessions: usize) -> Self {
        Self {
            manager: SessionManager::new(),
            order: VecDeque::new(),
            max_sessions,
        }
    }

    /// Return a reference to the history for `session_id`, or `None`.
    pub(super) fn get(&self, session_id: &str) -> Option<Vec<ChatMessage>> {
        self.manager
            .get_chat_history(session_id)
            .ok()
            .map(|messages| messages.into_iter().map(session_message_to_chat).collect())
    }

    /// Ensure a session exists and mark it as most-recently-used.
    pub(super) fn touch_or_create(&mut self, session_id: &str) {
        self.touch(session_id);
        if !self.manager.has_session(session_id) {
            self.manager
                .create_session_with_id(session_id.to_string(), "core", "core-default");
        }
    }

    /// Replace the full history for a session.
    pub(super) fn replace_history(&mut self, session_id: &str, messages: Vec<ChatMessage>) {
        self.touch_or_create(session_id);
        let _ = self.manager.clear_session(session_id);
        for message in messages {
            let _ = self
                .manager
                .add_message(session_id, chat_to_session_message(message));
        }
    }

    /// Get the underlying session manager.
    pub(super) fn manager(&self) -> &SessionManager {
        &self.manager
    }

    /// Append `messages` to `session_id`, creating the session if absent.
    pub(super) fn push_messages(
        &mut self,
        session_id: &str,
        messages: impl IntoIterator<Item = ChatMessage>,
    ) {
        self.touch_or_create(session_id);
        for message in messages {
            let _ = self
                .manager
                .add_message(session_id, chat_to_session_message(message));
        }
    }

    // ── internal helpers ─────────────────────────────────────────────────────

    /// Move `session_id` to the back of the access-order deque (most recent).
    ///
    /// If the session is new and the store is at capacity, the front entry
    /// (least recently used) is evicted first.
    fn touch(&mut self, session_id: &str) {
        if let Some(pos) = self.order.iter().position(|id| id == session_id) {
            self.order.remove(pos);
        } else if self.order.len() >= self.max_sessions
            && let Some(lru_id) = self.order.pop_front()
        {
            let _ = self.manager.delete_session(&lru_id);
            SessionLogger::new(&lru_id).debug(format!(
                "Evicted LRU session from in-memory store | evicted_session={} active_sessions={}",
                lru_id,
                self.order.len()
            ));
        }
        self.order.push_back(session_id.to_string());
    }
}

fn session_role_to_core(role: SessionMessageRole) -> MessageRole {
    match role {
        SessionMessageRole::System => MessageRole::System,
        SessionMessageRole::User => MessageRole::User,
        SessionMessageRole::Assistant => MessageRole::Assistant,
        SessionMessageRole::ToolResult => MessageRole::Assistant,
    }
}

fn core_role_to_session(role: MessageRole) -> SessionMessageRole {
    match role {
        MessageRole::System => SessionMessageRole::System,
        MessageRole::User => SessionMessageRole::User,
        MessageRole::Assistant => SessionMessageRole::Assistant,
        SessionMessageRole::ToolResult => SessionMessageRole::ToolResult,
    }
}

fn session_part_to_core(part: SessionMessagePart) -> MessagePart {
    match part {
        SessionMessagePart::Text { text } => MessagePart::text(text),
        SessionMessagePart::Image { mime_type, data } => MessagePart::image(mime_type, data),
        SessionMessagePart::File {
            name,
            mime_type,
            data,
        } => MessagePart::file(name, mime_type, data),
    }
}

fn core_part_to_session(part: MessagePart) -> SessionMessagePart {
    match part {
        MessagePart::Text { text } => SessionMessagePart::text(text),
        MessagePart::Image { mime_type, data } => SessionMessagePart::image(mime_type, data),
        MessagePart::File {
            name,
            mime_type,
            data,
        } => SessionMessagePart::file(name, mime_type, data),
    }
}

fn session_message_to_chat(message: Message) -> ChatMessage {
    let parts = if message.parts.is_empty() {
        vec![MessagePart::text(message.content)]
    } else {
        message
            .parts
            .into_iter()
            .map(session_part_to_core)
            .collect()
    };

    ChatMessage::with_parts(session_role_to_core(message.role), parts)
}

fn chat_to_session_message(message: ChatMessage) -> Message {
    Message::with_parts(
        core_role_to_session(message.role),
        message
            .parts
            .into_iter()
            .map(core_part_to_session)
            .collect(),
    )
}

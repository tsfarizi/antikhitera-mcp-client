use std::collections::{HashMap, VecDeque};

use crate::domain::types::ChatMessage;

/// Default maximum number of concurrent sessions kept in memory.
///
/// When this limit is reached the least-recently-used session is evicted
/// before a new one is created. This prevents unbounded memory growth in
/// long-running deployments with many ephemeral sessions.
pub(super) const DEFAULT_MAX_SESSIONS: usize = 256;

/// In-memory session store with LRU eviction.
pub(super) struct SessionStore {
    /// Session histories keyed by session_id.
    pub(super) map: HashMap<String, Vec<ChatMessage>>,
    /// Access order: front = least recently used, back = most recently used.
    pub(super) order: VecDeque<String>,
    /// Maximum number of sessions to retain simultaneously.
    pub(super) max_sessions: usize,
}

impl SessionStore {
    pub(super) fn new(max_sessions: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_sessions,
        }
    }

    /// Return a reference to the history for `session_id`, or `None`.
    pub(super) fn get(&self, session_id: &str) -> Option<&Vec<ChatMessage>> {
        self.map.get(session_id)
    }

    /// Return a mutable reference to the history, creating it if absent.
    ///
    /// Marks `session_id` as the most-recently-used and evicts the oldest
    /// session when the store is over capacity.
    pub(super) fn get_or_create(&mut self, session_id: &str) -> &mut Vec<ChatMessage> {
        self.touch(session_id);
        self.map.entry(session_id.to_string()).or_default()
    }

    /// Return a mutable reference to an existing session without creating one.
    pub(super) fn get_mut(&mut self, session_id: &str) -> Option<&mut Vec<ChatMessage>> {
        if self.map.contains_key(session_id) {
            self.touch(session_id);
        }
        self.map.get_mut(session_id)
    }

    /// Append `messages` to `session_id`, creating the session if absent.
    #[allow(dead_code)]
    pub(super) fn push_messages(
        &mut self,
        session_id: &str,
        messages: impl IntoIterator<Item = ChatMessage>,
    ) {
        let history = self.get_or_create(session_id);
        history.extend(messages);
    }

    /// Number of active sessions.
    #[allow(dead_code)]
    pub(super) fn len(&self) -> usize {
        self.map.len()
    }

    // ── internal helpers ─────────────────────────────────────────────────────

    /// Move `session_id` to the back of the access-order deque (most recent).
    ///
    /// If the session is new and the store is at capacity, the front entry
    /// (least recently used) is evicted first.
    fn touch(&mut self, session_id: &str) {
        if let Some(pos) = self.order.iter().position(|id| id == session_id) {
            self.order.remove(pos);
        } else if self.map.len() >= self.max_sessions
            && let Some(lru_id) = self.order.pop_front()
        {
            self.map.remove(&lru_id);
            tracing::debug!(
                evicted_session = %lru_id,
                active_sessions = self.map.len(),
                "Evicted LRU session from in-memory store"
            );
        }
        self.order.push_back(session_id.to_string());
    }
}

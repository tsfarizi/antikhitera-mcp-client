//! Client-side input streams

use std::collections::VecDeque;

/// Client-side input stream for large request payloads.
#[derive(Debug, Clone, Default)]
pub struct ClientInputStream {
    chunks: VecDeque<String>,
    is_complete: bool,
    total_chars_pushed: usize,
}

impl ClientInputStream {
    /// Create an empty, open input stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push the next input chunk.
    pub fn push_chunk(&mut self, chunk: impl Into<String>) {
        assert!(
            !self.is_complete,
            "cannot push to a completed ClientInputStream"
        );
        let s = chunk.into();
        self.total_chars_pushed += s.chars().count();
        self.chunks.push_back(s);
    }

    /// Signal end-of-stream.
    pub fn complete(&mut self) {
        self.is_complete = true;
    }

    /// Whether the producer has signalled end-of-stream.
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Pop the next available chunk.
    pub fn next_chunk(&mut self) -> Option<String> {
        self.chunks.pop_front()
    }

    /// Number of chunks waiting.
    pub fn pending_count(&self) -> usize {
        self.chunks.len()
    }

    /// Total characters pushed.
    pub fn total_chars_pushed(&self) -> usize {
        self.total_chars_pushed
    }

    /// Consume all chunks.
    pub fn collect_all(mut self) -> String {
        let mut result = String::with_capacity(self.total_chars_pushed);
        while let Some(chunk) = self.chunks.pop_front() {
            result.push_str(&chunk);
        }
        result
    }
}

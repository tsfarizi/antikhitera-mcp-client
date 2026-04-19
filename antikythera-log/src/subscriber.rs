//! Log Subscriber System
//!
//! Real-time log streaming via subscription.

use crate::entries::LogEntry;
use crate::logger::Logger;
use crossbeam_channel::{Receiver, Sender};

// ============================================================================
// Subscriber Types
// ============================================================================

/// Sender side (held by Logger)
pub type LogSender = Sender<LogEntry>;

/// Receiver side (given to subscriber)
pub type LogReceiver = Receiver<LogEntry>;

/// Log Subscriber
///
/// Receives log entries in real-time as they are written.
/// The subscriber is automatically removed when dropped.
pub struct LogSubscriber {
    receiver: LogReceiver,
}

impl LogSubscriber {
    pub(crate) fn new(logger: &Logger) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1000);

        let mut subscribers = logger.subscribers.lock().unwrap();
        subscribers.push(tx);

        Self { receiver: rx }
    }

    /// Receive next log entry (blocking)
    pub fn recv(&self) -> Result<LogEntry, crossbeam_channel::RecvError> {
        self.receiver.recv()
    }

    /// Receive next log entry (non-blocking)
    pub fn try_recv(&self) -> Result<LogEntry, crossbeam_channel::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Receive next log entry with timeout
    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<LogEntry, crossbeam_channel::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }

    /// Iterate over all available log entries
    pub fn iter(&self) -> crossbeam_channel::Iter<'_, LogEntry> {
        self.receiver.iter()
    }

    /// Check if there are pending log entries
    pub fn has_pending(&self) -> bool {
        !self.receiver.is_empty()
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.receiver.len()
    }
}

impl Iterator for LogSubscriber {
    type Item = LogEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.recv().ok()
    }
}

// ============================================================================
// Log Stream (for WASM/FFI compatibility)
// ============================================================================

/// Log stream for WASM environments (no threads)
#[cfg(not(feature = "subscriber"))]
pub struct LogStream {
    buffer: Vec<LogEntry>,
}

#[cfg(not(feature = "subscriber"))]
impl LogStream {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Get all logs
    pub fn get_logs(&self) -> &[LogEntry] {
        &self.buffer
    }

    /// Clear logs
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(not(feature = "subscriber"))]
impl Default for LogStream {
    fn default() -> Self {
        Self::new()
    }
}

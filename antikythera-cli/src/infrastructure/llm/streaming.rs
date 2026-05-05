//! Streaming event bus for live token delivery.
//!
//! Defines [`StreamEvent`] and a process-global sink ([`STREAM_SINK`]) that
//! provider parsers write to via [`emit_stream_event`].
//!
//! Two sinks are provided:
//! - [`install_terminal_stream_sink`] — writes chunks directly to stderr;
//!   used by the `--stream` CLI flag for line-oriented output.
//! - [`set_stream_event_sink`] — installs any custom callback; the TUI uses
//!   this to forward `Chunk` events over an `mpsc::unbounded_channel` so tokens
//!   appear in the Conversation panel while the request is still in flight.
//!
//! Call [`clear_stream_event_sink`] when the TUI or `--stream` session ends to
//! avoid leaking the channel sender across requests.

use std::io::Write;
use std::sync::{Arc, LazyLock, Mutex};

/// Streaming event emitted by provider parsers.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Started {
        provider_id: String,
        session_id: Option<String>,
    },
    Chunk {
        provider_id: String,
        session_id: Option<String>,
        content: String,
    },
    Completed {
        provider_id: String,
        session_id: Option<String>,
    },
}

pub type StreamEventSink = Arc<dyn Fn(&StreamEvent) + Send + Sync + 'static>;

static STREAM_SINK: LazyLock<Mutex<Option<StreamEventSink>>> = LazyLock::new(|| Mutex::new(None));

/// Install `sink` as the active [`STREAM_SINK`].
///
/// Replaces any previously installed sink.  The provided closure will be
/// called on every [`emit_stream_event`] call until [`clear_stream_event_sink`]
/// or the next [`set_stream_event_sink`] replaces it.
pub fn set_stream_event_sink(sink: StreamEventSink) {
    if let Ok(mut guard) = STREAM_SINK.lock() {
        *guard = Some(sink);
    }
}

/// Remove the active sink from [`STREAM_SINK`].
///
/// After this call, [`emit_stream_event`] is a no-op (events are silently
/// discarded).  The TUI calls this after a streaming response completes to
/// ensure the mpsc channel sender is dropped cleanly.
pub fn clear_stream_event_sink() {
    if let Ok(mut guard) = STREAM_SINK.lock() {
        *guard = None;
    }
}

/// Install a sink that prints stream events directly to `stderr`.
///
/// `Started` events print a header line; `Chunk` events write content
/// incrementally without a newline so the output looks like continuous
/// typing; `Completed` emits a trailing newline.  Enabled by the CLI
/// `--stream` flag for non-TUI streaming output.
pub fn install_terminal_stream_sink() {
    set_stream_event_sink(Arc::new(|event| match event {
        StreamEvent::Started { provider_id, .. } => {
            let _ = writeln!(std::io::stderr(), "\n[stream:{provider_id}] ");
        }
        StreamEvent::Chunk { content, .. } => {
            let _ = write!(std::io::stderr(), "{content}");
            let _ = std::io::stderr().flush();
        }
        StreamEvent::Completed { .. } => {
            let _ = writeln!(std::io::stderr());
        }
    }));
}

/// Action returned by a stream-line parser closure.
pub(crate) enum StreamAction {
    /// A content chunk extracted from the line.
    Chunk(String),
    /// Signal that the stream is complete — break the parsing loop.
    Done,
}

/// Shared SSE/NDJSON stream extraction loop.
///
/// Iterates over every line of `raw`, calling `parse_line` on each non-empty
/// line.  The closure may return:
///
/// | Return                | Behaviour                                    |
/// |-----------------------|----------------------------------------------|
/// | `Some(Chunk(s))`      | Emit `StreamEvent::Chunk`, accumulate `s`.   |
/// | `Some(Done)`          | Break the loop.                              |
/// | `None`                | Skip the line.                               |
///
/// Emits `Started` before the first line and `Completed` after the loop exits.
pub(crate) fn extract_stream_content(
    raw: &str,
    provider_id: &str,
    session_id: Option<&str>,
    parse_line: impl Fn(&str) -> Option<StreamAction>,
) -> Option<String> {
    let mut content = String::new();
    let mut saw_chunk = false;
    let session = session_id.map(|v| v.to_string());

    emit_stream_event(StreamEvent::Started {
        provider_id: provider_id.to_string(),
        session_id: session.clone(),
    });

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match parse_line(trimmed) {
            Some(StreamAction::Chunk(piece)) => {
                saw_chunk = true;
                emit_stream_event(StreamEvent::Chunk {
                    provider_id: provider_id.to_string(),
                    session_id: session.clone(),
                    content: piece.clone(),
                });
                content.push_str(&piece);
            }
            Some(StreamAction::Done) => break,
            None => {}
        }
    }

    emit_stream_event(StreamEvent::Completed {
        provider_id: provider_id.to_string(),
        session_id: session,
    });

    if saw_chunk { Some(content) } else { None }
}

pub(crate) fn emit_stream_event(event: StreamEvent) {
    // Emit log entries for Start and Completion so they appear in the log
    // panel under the "cli:streaming" source label.  Chunks are intentionally
    // skipped here — they are too frequent and the content appears in the chat
    // area anyway.
    match &event {
        StreamEvent::Started {
            provider_id,
            session_id,
        } => {
            antikythera_core::StreamingLogger::new(session_id.as_deref().unwrap_or("tui")).info(
                format!(
                    "Stream started | provider={} session={}",
                    provider_id.as_str(),
                    session_id.as_deref().unwrap_or("-")
                ),
            );
        }
        StreamEvent::Completed {
            provider_id,
            session_id,
        } => {
            antikythera_core::StreamingLogger::new(session_id.as_deref().unwrap_or("tui")).info(
                format!(
                    "Stream completed | provider={} session={}",
                    provider_id.as_str(),
                    session_id.as_deref().unwrap_or("-")
                ),
            );
        }
        StreamEvent::Chunk { .. } => {}
    }
    if let Ok(guard) = STREAM_SINK.lock()
        && let Some(sink) = guard.as_ref()
    {
        sink(&event);
    }
}

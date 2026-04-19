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

static STREAM_SINK: LazyLock<Mutex<Option<StreamEventSink>>> =
    LazyLock::new(|| Mutex::new(None));

pub fn set_stream_event_sink(sink: StreamEventSink) {
    if let Ok(mut guard) = STREAM_SINK.lock() {
        *guard = Some(sink);
    }
}

pub fn clear_stream_event_sink() {
    if let Ok(mut guard) = STREAM_SINK.lock() {
        *guard = None;
    }
}

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

pub(crate) fn emit_stream_event(event: StreamEvent) {
    if let Ok(guard) = STREAM_SINK.lock() {
        if let Some(sink) = guard.as_ref() {
            sink(&event);
        }
    }
}

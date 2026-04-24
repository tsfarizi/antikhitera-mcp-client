//! TUI Tracing Bridge
//!
//! A `tracing_subscriber::Layer` that routes all `tracing::debug!` / `info!` /
//! `warn!` / `error!` events produced by antikythera-core (client, providers,
//! agent FSM, model HTTP clients, etc.) into the antikythera LOGGERS system.
//!
//! Without this bridge the WASM/FFI Logs panel is always empty because the core
//! codebase uses the standard `tracing` crate while the TUI panel reads from
//! the custom `LOGGERS` HashMap — two systems that have no connection by default.
//!
//! Usage in `menu.rs`:
//! ```ignore
//! use antikythera_cli::presentation::tui_tracing::AntikytheraTuiLayer;
//! use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
//!
//! tracing_subscriber::registry()
//!     .with(AntikytheraTuiLayer)
//!     .init();
//! ```

use antikythera_core::{LogLevel, get_active_session, get_logger};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// Routes tracing events to the antikythera LOGGERS system under the current
/// active session (defaulting to "tui"). Installed once at process startup.
pub struct AntikytheraTuiLayer;

impl<S: Subscriber> Layer<S> for AntikytheraTuiLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Skip TRACE level — too verbose for the panel.
        let level = match *event.metadata().level() {
            tracing::Level::ERROR => LogLevel::Error,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::TRACE => return,
        };

        // Derive a categorised source label that tells the operator which
        // layer of the stack emitted the event:
        //   cli:*     — antikythera-cli crate (HTTP clients, streaming, factory)
        //   ffi:*     — WASM/FFI infrastructure (wasm runner, host functions)
        //   stream:*  — Model HTTP clients (request/response to Ollama/Gemini/OpenAI)
        //   agent:*   — Agent FSM, runner, context, parser, multi-agent
        //   tool:*    — Tooling layer (transport, SSE, RPC, process manager)
        //   core:*    — Everything else in antikythera-core
        let target = event.metadata().target();
        let source = categorize_source(target);

        // Collect message and additional key-value fields.
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let message = visitor.finish();
        if message.is_empty() {
            return;
        }

        let session_id = get_active_session();
        get_logger(&session_id).log_with_source(level, source, message);
    }
}

/// Map a tracing target path to a categorised source label.
fn categorize_source(target: &str) -> String {
    let last = target.rsplit("::").next().unwrap_or(target);
    if target.starts_with("antikythera_cli::") {
        // CLI crate events — distinguish from identically-named core modules.
        format!("cli:{last}")
    } else if target.starts_with("antikythera_core::infrastructure::wasm") {
        // WASM / FFI host-side events.
        format!("ffi:{last}")
    } else if target.starts_with("antikythera_core::infrastructure::model") {
        // Model HTTP clients — the actual LLM API calls and streaming.
        format!("stream:{last}")
    } else if target.starts_with("antikythera_core::application::agent") {
        // Agent FSM, runner, tool-call execution, response parser.
        format!("agent:{last}")
    } else if target.starts_with("antikythera_core::application::tooling") {
        // MCP tool transports: SSE, JSON-RPC, process spawning.
        format!("tool:{last}")
    } else {
        // antikythera_core::application::client, services, discovery, etc.
        format!("core:{last}")
    }
}

// ── Field visitor ─────────────────────────────────────────────────────────────

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl MessageVisitor {
    /// Combine message + extra fields into a single display string.
    fn finish(self) -> String {
        let base = self.message.unwrap_or_default();
        if self.fields.is_empty() {
            base
        } else {
            let extras = self
                .fields
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ");
            if base.is_empty() {
                extras
            } else {
                format!("{base} | {extras}")
            }
        }
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let rendered = format!("{value:?}");
        // Remove surrounding quotes that Debug adds to strings.
        let clean = rendered
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .map(|s| s.to_string())
            .unwrap_or(rendered);

        if field.name() == "message" {
            self.message = Some(clean);
        } else {
            self.fields.push((field.name().to_string(), clean));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }
}

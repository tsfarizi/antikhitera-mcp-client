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

use antikythera_core::{get_active_session, get_logger, LogLevel};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

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

        // Use the module path as a short source label.
        // e.g. "antikythera_core::application::client" → "client"
        let target = event.metadata().target();
        let source = target.rsplit("::").next().unwrap_or(target);

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
            self.fields.push((field.name().to_string(), value.to_string()));
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let rendered = format!("{value:?}");
        // Remove surrounding quotes that Debug adds to strings.
        let clean = rendered.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
            .map(|s| s.to_string())
            .unwrap_or(rendered);

        if field.name() == "message" {
            self.message = Some(clean);
        } else {
            self.fields.push((field.name().to_string(), clean));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.push((field.name().to_string(), value.to_string()));
    }
}

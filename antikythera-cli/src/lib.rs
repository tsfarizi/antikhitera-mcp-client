//! Antikythera CLI Crate
//!
//! Organises the CLI binary in **Vertical Slice Architecture** (VSA).
//! Each feature slice is self-contained — its domain logic, infrastructure
//! adapters, and presentation code live together rather than being spread
//! across horizontal layers.
//!
//! ## Feature Slices
//!
//! | Slice | Module path | Description |
//! |-------|------------|-------------|
//! | **Chat** | `domain/use_cases/chat_use_case` + `presentation/tui` | Interactive TUI chat with tool-call agent loop |
//! | **WASM Harness** | `domain/use_cases/wasm_harness_use_case` | Diagnostic probe for the WASM/FFI SDK surface |
//! | **Config** | `config` + `infrastructure/config` | Read/write the unified `app.pc` configuration |
//! | **Runtime** | `runtime` | Provider auto-detection and `McpClient` wiring |
//!
//! ## Shared Infrastructure
//!
//! - `infrastructure/llm/` — LLM provider adapters (Gemini, Ollama, OpenAI)
//! - `error` — `CliError` and `CliResult` used by all slices
//! - `cli` — Clap argument parser (`Cli` struct, `RunMode` enum)

// Domain layer: entities (shared types) + per-slice use cases
pub mod domain;

// Infrastructure layer: LLM adapters and config loader implementations
pub mod infrastructure;

// Shared error contract for all feature slices.
pub mod error;

// Chat slice — presentation (full-screen TUI)
pub mod presentation;

// Config slice — shared AppConfig type and serialization helpers.
pub mod config;

// Runtime slice — provider auto-detection and McpClient wiring.
pub mod runtime;

// CLI argument parsing (owned by CLI crate, not core).
pub mod cli;

// Re-exports for convenience
pub use config::CliConfig;
pub use error::{CliError, CliResult};

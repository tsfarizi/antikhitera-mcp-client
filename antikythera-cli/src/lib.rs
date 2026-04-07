//! Antikythera CLI
//!
//! Clean Architecture:
//! - domain/ (core entities & use cases)
//! - infrastructure/ (LLM providers, config loading)
//! - presentation/ (TUI)
//!
//! CLI acts as the "host" - it calls LLM APIs directly (native binary).
//! WASM receives LLM responses from host via FFI (WASM binary).

// Domain layer (innermost, no external deps)
pub mod domain;

// Infrastructure layer (implements domain ports)
pub mod infrastructure;

// Presentation layer (TUI)
pub mod presentation;

// Config module (for CLI testing)
pub mod config;

// Re-exports for convenience
pub use config::CliConfig;

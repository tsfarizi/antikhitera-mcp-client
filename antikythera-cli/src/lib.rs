//! Antikythera CLI
//!
//! Clean Architecture:
//! - domain/ (core entities & use cases)
//! - infrastructure/ (LLM providers, config loading)
//! - presentation/ (TUI)
//!
//! CLI acts as a thin local surface over the framework runtime.
//! Model API invocation is host-owned and no longer implemented inside this repository.
//! Any host, including a native CLI shell, must call the LLM externally and feed the
//! result back through the framework/session boundary.

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

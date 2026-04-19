//! LLM infrastructure compatibility layer.
//!
//! Direct model API calls are no longer implemented in this repository.
//! These modules are kept only so older call sites fail explicitly with a
//! clear error message instead of silently performing network I/O.

pub mod gemini;
pub mod ollama;
pub mod provider_builder;

pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use provider_builder::build_provider_from_configs;

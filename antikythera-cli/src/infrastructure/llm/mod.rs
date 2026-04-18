//! LLM infrastructure — CLI-side implementations
//!
//! This module owns all HTTP LLM provider logic so that `antikythera-core`
//! can be compiled to WASM without any HTTP client dependencies.
//!
//! # Module layout
//!
//! - `adapter`           — message-format converters (OpenAI / Ollama / Gemini wire formats)
//! - `http_client`       — shared `HttpClientBase` for async HTTP calls
//! - `clients`           — concrete per-provider `ModelClient` implementations
//! - `factory`           — `ProviderFactory::create()` dispatching on provider type
//! - `provider_builder`  — builds a `DynamicModelProvider` from config slices
//! - `gemini`            — thin `LlmProvider` wrapper for the chat use-case
//! - `ollama`            — thin `LlmProvider` wrapper for the chat use-case

pub mod adapter;
pub mod clients;
pub mod factory;
pub mod gemini;
pub mod http_client;
pub mod ollama;
pub mod provider_builder;

pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use provider_builder::build_provider_from_configs;

//! LLM infrastructure layer for native CLI runtime.

mod adapter;
mod clients;
mod factory;
mod http_client;
mod streaming;
pub mod types;

pub mod gemini;
pub mod ollama;
pub mod provider_builder;

pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use provider_builder::build_provider_from_configs;
pub use streaming::{
    StreamEvent, clear_stream_event_sink, install_terminal_stream_sink, set_stream_event_sink,
};
pub use types::{
    ModelInfo, ModelProviderConfig, providers_from_postcard, providers_to_postcard,
};

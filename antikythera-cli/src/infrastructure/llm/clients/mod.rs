//! LLM client implementations — CLI-side
//!
//! These are the concrete HTTP client implementations for each LLM provider.
//! They live in `antikythera-cli` so that `antikythera-core` can be compiled
//! to WASM without any HTTP client code.
//!
//! Each client implements `antikythera_core::infrastructure::model::traits::ModelClient`.

mod gemini;
mod ollama;
mod openai;

pub use gemini::GeminiClient;
pub use ollama::OllamaClient;
pub use openai::OpenAIClient;

//! LLM Provider implementations
//!
//! Only GEMINI and OLLAMA supported.

pub mod gemini;
pub mod ollama;

pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;

//! Model clients

mod base;
mod gemini;
mod ollama;
mod openai;

pub use base::HttpClientBase;
pub use gemini::GeminiClient;
pub use ollama::OllamaClient;
pub use openai::OpenAIClient;

//! CLI Infrastructure Layer
//!
//! External services: LLM providers, MCP clients, config loading.
//! Implements the domain ports (interfaces).

pub mod llm;
pub mod config;

pub use llm::*;
pub use config::*;

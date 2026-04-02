//! # Antikythera Core
//!
//! Core MCP protocol implementation, transport layers, and agent runtime.

pub mod application;
pub mod cli;
pub mod config;
pub mod constants;
pub mod domain;
pub mod infrastructure;

// Re-export commonly used types
pub use application::agent::{Agent, AgentOptions, AgentOutcome, ToolDescriptor};
pub use application::client::{ChatRequest, ClientConfig, McpClient};
pub use config::AppConfig;
pub use infrastructure::model::{DynamicModelProvider, ModelProvider};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

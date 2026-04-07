//! Multi-Agent support module

pub mod registry;

// Re-exports for SDK compatibility
pub use registry::{AgentProfile, AgentRole, AgentRegistry, MemoryProvider, MemoryConfig, ContextId};

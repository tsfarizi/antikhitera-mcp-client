//! Re-export CLI domain entities from core to avoid cross-crate drift.
//!
//! The CLI crate is a presentation/adaptor layer and should not redefine
//! canonical domain entities already owned by `antikythera-core`.

pub use antikythera_core::domain::entities::{
    AgentAction, ChatSession, Message, MessageRole, ProviderConfig, ProviderType, ToolCall,
    ToolResult,
};

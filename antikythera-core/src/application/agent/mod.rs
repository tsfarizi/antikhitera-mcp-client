//! # Agent Module
//!
//! This module provides an autonomous AI agent that can use tools and execute
//! multi-step tasks. The agent implements a feedback loop with JSON retry logic
//! for robust LLM interaction.
//!
//! ## Key Types
//!
//! - [`Agent`] - The main agent executor
//! - [`AgentOptions`] - Configuration options for agent behavior
//! - [`AgentOutcome`] - Result of agent execution
//! - [`ToolContext`] - Context passed to tools during execution
//! - [`AgentError`] - Errors that can occur during agent execution
//!
//! ## Agent Loop
//!
//! The agent operates in a loop:
//! 1. Send messages to LLM
//! 2. Parse JSON response (with retry on parse failure)
//! 3. If tool call requested, execute tool and continue
//! 4. If final response, return to user

mod context;
mod directive;
mod errors;
mod fsm_runner;
mod memory;
mod models;
mod runner;
mod runtime;
mod state;

#[cfg(feature = "multi-agent")]
pub mod multi_agent;

pub use context::{ServerGuidance, ToolContext, ToolDescriptor};
pub use errors::{AgentError, ToolError};
pub use fsm_runner::FsmAgent;
pub use memory::{
    AgentStateSnapshot, Attachment, ConversationTurn, MemoryError,
    MemoryProvider, StateMetadata, STATE_SCHEMA_VERSION,
};
pub use models::{AgentOptions, AgentOutcome, AgentStep};
pub use runner::Agent;
pub use state::{AgentState, Event, TerminationReason};
#[allow(unused_imports)]
pub(crate) use runtime::ToolRuntime;

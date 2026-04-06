//! Agent Finite State Machine (FSM) State Definitions
//!
//! This module defines the states and events for the agent FSM.
//! Currently a stub - full implementation pending.

use serde::{Deserialize, Serialize};

/// Agent states for FSM-driven execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Initial state
    Idle,
    /// Parsing LLM response
    ParsingDirective,
    /// Executing a tool
    ExecutingTool { tool_id: String, input: serde_json::Value },
    /// Waiting for external context
    WaitingForContext,
    /// Recovering from error
    RecoveringError { error: String, retry_count: u8 },
    /// Finalizing response
    FinalizingResponse,
    /// ⭐ NEW: Final message with formatted JSON response
    /// This state holds the AI's final response after it has been
    /// parsed and formatted as a proper JSON object (not a tool call)
    FinalMessage {
        /// The final response content from AI
        content: String,
        /// Optional structured data extracted from response
        data: Option<serde_json::Value>,
        /// Response metadata (tokens, model, etc.)
        metadata: Option<serde_json::Value>,
    },
    /// Terminated
    Terminated { reason: TerminationReason },
}

/// Termination reasons
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminationReason {
    Success,
    Error { message: String },
    MaxStepsExceeded,
    Timeout,
    Cancelled,
}

/// Events that trigger state transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    PromptReceived { prompt: String },
    DirectiveParsed { tool: String, input: serde_json::Value },
    DirectivesParsed { tools: Vec<(String, serde_json::Value)> },
    FinalResponse,
    ToolCompleted { tool: String, output: serde_json::Value },
    ToolsCompleted { results: Vec<Result<ToolResult, String>> },
    ToolFailed { tool: String, error: String },
    ContextReceived { context: String },
    ResponseSent,
    MaxStepsExceeded,
    Timeout,
    Error { message: String },
    Cancelled,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool: String,
    pub input: serde_json::Value,
    pub success: bool,
    pub output: serde_json::Value,
    pub message: Option<String>,
}

impl AgentState {
    /// Check if state is terminal
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentState::Terminated { .. })
    }

    /// Transition to next state based on event
    pub fn transition(self, _event: Event) -> Self {
        // TODO: Implement full FSM transition logic
        // For now, return current state as placeholder
        self
    }
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Idle => write!(f, "Idle"),
            AgentState::ParsingDirective => write!(f, "ParsingDirective"),
            AgentState::ExecutingTool { .. } => write!(f, "ExecutingTool"),
            AgentState::WaitingForContext => write!(f, "WaitingForContext"),
            AgentState::RecoveringError { .. } => write!(f, "RecoveringError"),
            AgentState::FinalizingResponse => write!(f, "FinalizingResponse"),
            AgentState::FinalMessage { .. } => write!(f, "FinalMessage"),
            AgentState::Terminated { .. } => write!(f, "Terminated"),
        }
    }
}

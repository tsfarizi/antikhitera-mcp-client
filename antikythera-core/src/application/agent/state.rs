//! Agent Finite State Machine (FSM) State Definitions
//!
//! This module defines the states, events, and transition logic for the agent FSM.
//! Every state/event pair maps to a well-defined next state; unrecognised pairs are
//! no-ops (the state is returned unchanged) so callers never receive an invalid state.

use serde::{Deserialize, Serialize};

/// Agent states for FSM-driven execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Initial state
    Idle,
    /// Parsing LLM response
    ParsingDirective,
    /// Executing a tool
    ExecutingTool {
        tool_id: String,
        input: serde_json::Value,
    },
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
    PromptReceived {
        prompt: String,
    },
    DirectiveParsed {
        tool: String,
        input: serde_json::Value,
    },
    DirectivesParsed {
        tools: Vec<(String, serde_json::Value)>,
    },
    FinalResponse,
    ToolCompleted {
        tool: String,
        output: serde_json::Value,
    },
    ToolsCompleted {
        results: Vec<Result<ToolResult, String>>,
    },
    ToolFailed {
        tool: String,
        error: String,
    },
    ContextReceived {
        context: String,
    },
    ResponseSent,
    MaxStepsExceeded,
    Timeout,
    Error {
        message: String,
    },
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
    /// Returns `true` when the agent has reached a terminal state and the FSM
    /// loop should exit.  Both `FinalMessage` (successful response) and
    /// `Terminated` (error / cancelled / timed-out) are terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentState::Terminated { .. } | AgentState::FinalMessage { .. }
        )
    }

    /// Apply `event` to the current state and return the next state.
    ///
    /// Transitions are deterministic: every valid `(state, event)` pair maps to
    /// a specific successor state.  Invalid pairs leave the state unchanged so
    /// the caller always receives a coherent state.
    pub fn transition(self, event: Event) -> Self {
        match (self, event) {
            // ── Idle ──────────────────────────────────────────────────────────
            (AgentState::Idle, Event::PromptReceived { .. }) => AgentState::ParsingDirective,
            (AgentState::Idle, Event::Cancelled) => AgentState::Terminated {
                reason: TerminationReason::Cancelled,
            },
            (AgentState::Idle, Event::Error { message }) => AgentState::RecoveringError {
                error: message,
                retry_count: 0,
            },

            // ── ParsingDirective ──────────────────────────────────────────────
            (AgentState::ParsingDirective, Event::DirectiveParsed { tool, input }) => {
                AgentState::ExecutingTool {
                    tool_id: tool,
                    input,
                }
            }
            (AgentState::ParsingDirective, Event::DirectivesParsed { tools }) => {
                if let Some((tool, input)) = tools.into_iter().next() {
                    AgentState::ExecutingTool {
                        tool_id: tool,
                        input,
                    }
                } else {
                    AgentState::FinalizingResponse
                }
            }
            (AgentState::ParsingDirective, Event::FinalResponse) => AgentState::FinalizingResponse,
            (AgentState::ParsingDirective, Event::Error { message }) => {
                AgentState::RecoveringError {
                    error: message,
                    retry_count: 0,
                }
            }
            (AgentState::ParsingDirective, Event::MaxStepsExceeded) => AgentState::Terminated {
                reason: TerminationReason::MaxStepsExceeded,
            },
            (AgentState::ParsingDirective, Event::Timeout) => AgentState::Terminated {
                reason: TerminationReason::Timeout,
            },
            (AgentState::ParsingDirective, Event::Cancelled) => AgentState::Terminated {
                reason: TerminationReason::Cancelled,
            },

            // ── ExecutingTool ─────────────────────────────────────────────────
            (AgentState::ExecutingTool { .. }, Event::ToolCompleted { .. }) => {
                AgentState::ParsingDirective
            }
            (AgentState::ExecutingTool { .. }, Event::ToolsCompleted { .. }) => {
                AgentState::ParsingDirective
            }
            (AgentState::ExecutingTool { .. }, Event::ToolFailed { error, .. }) => {
                AgentState::RecoveringError {
                    error,
                    retry_count: 0,
                }
            }
            (AgentState::ExecutingTool { .. }, Event::Error { message }) => {
                AgentState::RecoveringError {
                    error: message,
                    retry_count: 0,
                }
            }
            (AgentState::ExecutingTool { .. }, Event::MaxStepsExceeded) => AgentState::Terminated {
                reason: TerminationReason::MaxStepsExceeded,
            },
            (AgentState::ExecutingTool { .. }, Event::Timeout) => AgentState::Terminated {
                reason: TerminationReason::Timeout,
            },
            (AgentState::ExecutingTool { .. }, Event::Cancelled) => AgentState::Terminated {
                reason: TerminationReason::Cancelled,
            },

            // ── WaitingForContext ─────────────────────────────────────────────
            (AgentState::WaitingForContext, Event::ContextReceived { .. }) => {
                AgentState::ParsingDirective
            }
            (AgentState::WaitingForContext, Event::Timeout) => AgentState::Terminated {
                reason: TerminationReason::Timeout,
            },
            (AgentState::WaitingForContext, Event::Cancelled) => AgentState::Terminated {
                reason: TerminationReason::Cancelled,
            },
            (AgentState::WaitingForContext, Event::Error { message }) => {
                AgentState::RecoveringError {
                    error: message,
                    retry_count: 0,
                }
            }

            // ── RecoveringError ───────────────────────────────────────────────
            // Another error while recovering increments the retry counter.
            (AgentState::RecoveringError { error, retry_count }, Event::Error { .. }) => {
                AgentState::RecoveringError {
                    error,
                    retry_count: retry_count.saturating_add(1),
                }
            }
            // A new prompt resets recovery and resumes directive parsing.
            (AgentState::RecoveringError { .. }, Event::PromptReceived { .. }) => {
                AgentState::ParsingDirective
            }
            (AgentState::RecoveringError { error, .. }, Event::Timeout) => AgentState::Terminated {
                reason: TerminationReason::Error { message: error },
            },
            (AgentState::RecoveringError { error, .. }, Event::MaxStepsExceeded) => {
                AgentState::Terminated {
                    reason: TerminationReason::Error { message: error },
                }
            }
            (AgentState::RecoveringError { .. }, Event::Cancelled) => AgentState::Terminated {
                reason: TerminationReason::Cancelled,
            },

            // ── FinalizingResponse ────────────────────────────────────────────
            (AgentState::FinalizingResponse, Event::ResponseSent) => AgentState::FinalMessage {
                content: String::new(),
                data: None,
                metadata: None,
            },
            (AgentState::FinalizingResponse, Event::Timeout) => AgentState::Terminated {
                reason: TerminationReason::Timeout,
            },
            (AgentState::FinalizingResponse, Event::Cancelled) => AgentState::Terminated {
                reason: TerminationReason::Cancelled,
            },
            (AgentState::FinalizingResponse, Event::Error { message }) => {
                AgentState::RecoveringError {
                    error: message,
                    retry_count: 0,
                }
            }

            // ── FinalMessage – sticky terminal ────────────────────────────────
            (state @ AgentState::FinalMessage { .. }, _) => state,

            // ── Terminated – sticky terminal ──────────────────────────────────
            (state @ AgentState::Terminated { .. }, _) => state,

            // ── Unrecognised (state, event) – no-op ───────────────────────────
            (state, _) => state,
        }
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


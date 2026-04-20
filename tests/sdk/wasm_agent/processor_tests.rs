//! Centralized unit tests for the WASM agent processor.
//!
//! Validates the generic JSON format contract (the only format WASM now accepts)
//! and the plain-text fallback.  Provider-native formats (OpenAI, Gemini,
//! Anthropic) are intentionally **not** tested here â€” that parsing is the
//! host's responsibility via FFI.

use antikythera_sdk::{
    AgentAction, AgentState, ToolDefinition, ToolParameterSchema, ToolRegistry,
    ToolValidationError, WasmAgentConfig, process_llm_response, validate_tool_call,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fresh_state() -> AgentState {
    AgentState::new(WasmAgentConfig::default())
}

// ---------------------------------------------------------------------------
// 1. Generic call_tool format
// ---------------------------------------------------------------------------

// Split into 5 parts for consistent test organization.
include!("processor_tests/part_01.rs");
include!("processor_tests/part_02.rs");
include!("processor_tests/part_03.rs");
include!("processor_tests/part_04.rs");
include!("processor_tests/part_05.rs");

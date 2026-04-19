//! WASM Agent Module
//!
//! WASM component that processes LLM responses from host.
//! WASM does NOT call LLM APIs directly.
//!
//! ## Architecture
//!
//! ```text
//! wasm_agent/
//! ├── types.rs       # Agent types (state, messages, actions)
//! ├── processor.rs   # LLM response processing logic
//! └── mod.rs         # Module exports
//! ```
//!
//! ## Host Responsibility
//!
//! The host (TypeScript/Python/Go) handles:
//! - LLM API calls (OpenAI, Anthropic, Gemini, Ollama)
//! - API key management
//! - Rate limiting & retry logic
//! - MCP tool execution
//!
//! ## WASM Responsibility
//!
//! WASM handles:
//! - Agent FSM logic
//! - JSON parsing & validation
//! - Schema enforcement
//! - Prompt building
//! - Tool call extraction

pub mod types;
pub mod processor;
pub mod runner;

// Re-export main types
pub use types::{
    AgentAction,
    AgentState,
    AgentConfig as WasmAgentConfig,
    AgentMessage,
    ToolCall,
    ToolResult,
    PromptVariables,
};

pub use processor::{
    process_llm_response,
    process_tool_result,
    build_system_prompt,
    build_llm_messages,
    validate_json_schema,
};

pub use runner::{
    init,
    prepare_user_turn,
    commit_llm_response,
    process_llm_response_for_session,
    process_tool_result_for_session,
    get_state,
    reset_session,
};

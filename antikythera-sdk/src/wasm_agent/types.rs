//! WASM Agent Types
//!
//! Types for WASM agent that processes LLM responses.
//! WASM does NOT call LLM APIs - host does that.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Agent Actions
// ============================================================================

/// Action the agent wants to take
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentAction {
    /// Call a tool
    CallTool {
        tool: String,
        input: serde_json::Value,
    },
    /// Final response to user
    Final { response: serde_json::Value },
    /// Retry with error
    Retry { error: String },
}

// ============================================================================
// Advanced Context Management
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TruncationStrategy {
    #[default]
    KeepNewest,
    KeepBalanced,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextPolicy {
    pub max_history_messages: usize,
    pub summarize_after_messages: usize,
    pub summary_max_chars: usize,
    #[serde(default)]
    pub truncation_strategy: TruncationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextSummary {
    pub version: u64,
    pub text: String,
    pub source_messages: usize,
}

// ============================================================================
// Streaming and Telemetry
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamEventKind {
    UserTurnPrepared,
    LlmChunk,
    LlmCommitted,
    ToolRequested,
    ToolResult,
    FinalResponse,
    SummaryUpdated,
    SessionArchived,
    SessionRestoreRequested,
    SessionRestoreProgress,
    SessionRestored,
    Telemetry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub seq: u64,
    pub session_id: String,
    pub step: u32,
    pub correlation_id: Option<String>,
    pub kind: StreamEventKind,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryCounters {
    pub turns_prepared: u64,
    pub llm_chunks: u64,
    pub llm_commits: u64,
    pub llm_retries: u64,
    pub tool_requests: u64,
    pub tool_results: u64,
    pub tool_errors: u64,
    pub final_responses: u64,
    pub context_summaries: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetrySnapshot {
    pub session_id: String,
    pub correlation_id: Option<String>,
    pub counters: TelemetryCounters,
    pub total_prepare_latency_ms: u64,
    pub total_commit_latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SloSnapshot {
    pub session_id: String,
    pub correlation_id: Option<String>,
    pub success_rate: f64,
    pub tool_error_rate: f64,
    pub retry_ratio: f64,
    pub p95_prepare_latency_ms: u64,
    pub p95_commit_latency_ms: u64,
}

// ============================================================================
// Agent State
// ============================================================================

/// Agent session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Session ID
    pub session_id: String,
    /// Current step number
    pub current_step: u32,
    /// Message history (user + assistant + tool results)
    pub message_history: Vec<AgentMessage>,
    /// Tool call results
    pub tool_results: HashMap<String, serde_json::Value>,
    /// Agent configuration
    pub config: AgentConfig,
    /// Rolling summary for long context
    #[serde(default)]
    pub rolling_summary: Option<ContextSummary>,
}

impl AgentState {
    /// Create new session
    pub fn new(config: AgentConfig) -> Self {
        Self {
            session_id: config.session_id.clone(),
            current_step: 0,
            message_history: Vec::new(),
            tool_results: HashMap::new(),
            config,
            rolling_summary: None,
        }
    }

    /// Add message to history
    pub fn add_message(&mut self, message: AgentMessage) {
        self.message_history.push(message);
    }

    /// Record tool result
    pub fn record_tool_result(&mut self, tool_name: String, result: serde_json::Value) {
        self.tool_results.insert(tool_name, result);
        self.current_step += 1;
    }

    /// Check if max steps exceeded
    pub fn is_max_steps_exceeded(&self) -> bool {
        self.current_step >= self.config.max_steps
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Serialize error: {}", e))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Deserialize error: {}", e))
    }
}

// ============================================================================
// Messages
// ============================================================================

/// Message in conversation (for WASM agent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Role (user, assistant, system, tool)
    pub role: String,
    /// Message content
    pub content: String,
    /// Optional tool call info
    pub tool_call: Option<ToolCall>,
    /// Optional tool result
    pub tool_result: Option<ToolResult>,
}

/// Tool call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name
    pub name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
    /// Step ID
    pub step_id: u32,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool name
    pub name: String,
    /// Success status
    pub success: bool,
    /// Output
    pub output: serde_json::Value,
    /// Error message
    pub error: Option<String>,
    /// Step ID
    pub step_id: u32,
}

// ============================================================================
// Tool Registry (MCP tool definitions for WASM-side validation)
// ============================================================================

/// A single parameter definition within a tool's input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameterSchema {
    /// Parameter name
    pub name: String,
    /// JSON Schema type string (e.g. "string", "number", "object", "array")
    pub param_type: String,
    /// Human-readable description
    pub description: String,
    /// Whether this parameter is required
    pub required: bool,
}

/// Definition of a single MCP tool, pushed from host to WASM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name as exposed by MCP server
    pub name: String,
    /// Human-readable description shown to the LLM
    pub description: String,
    /// Individual parameter schemas (may be empty if raw input_schema is used)
    #[serde(default)]
    pub parameters: Vec<ToolParameterSchema>,
    /// Full JSON Schema for the input object (optional; takes precedence for validation)
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
}

impl ToolDefinition {
    /// Returns names of required parameters derived from `input_schema` or `parameters`.
    pub fn required_params(&self) -> Vec<&str> {
        if let Some(schema) = &self.input_schema
            && let Some(required) = schema.get("required").and_then(|v| v.as_array())
        {
            return required.iter().filter_map(|v| v.as_str()).collect();
        }
        self.parameters
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.as_str())
            .collect()
    }

    /// Renders a compact text line for LLM prompt injection.
    pub fn to_prompt_line(&self) -> String {
        let params: Vec<String> = if let Some(schema) = &self.input_schema {
            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                props.keys().cloned().collect()
            } else {
                Vec::new()
            }
        } else {
            self.parameters.iter().map(|p| p.name.clone()).collect()
        };

        let required = self.required_params();
        let param_display: Vec<String> = params
            .iter()
            .map(|p| {
                if required.contains(&p.as_str()) {
                    format!("{}*", p)
                } else {
                    p.clone()
                }
            })
            .collect();

        if param_display.is_empty() {
            format!("- `{}`: {}", self.name, self.description)
        } else {
            format!(
                "- `{}` ({}): {}",
                self.name,
                param_display.join(", "),
                self.description
            )
        }
    }
}

/// Error from validating a tool call against the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolValidationError {
    /// Tool name not found in registry (registry must be non-empty to trigger this)
    UnknownTool { name: String },
    /// A required parameter is absent from the call arguments
    MissingRequiredParam { tool: String, param: String },
}

impl std::fmt::Display for ToolValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTool { name } => write!(f, "Unknown tool: '{}'", name),
            Self::MissingRequiredParam { tool, param } => {
                write!(f, "Tool '{}': missing required param '{}'", tool, param)
            }
        }
    }
}

/// WASM-side registry of tools available from the MCP server.
///
/// Populated by the host via `register_tools()` before or during session init.
/// When non-empty, all `CallTool` actions are validated against this registry.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// Register a tool definition.
    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Returns `true` if the registry has been populated with at least one tool.
    pub fn is_populated(&self) -> bool {
        !self.tools.is_empty()
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// Validate a tool call: checks unknown tool and missing required params.
    pub fn validate_call(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<(), ToolValidationError> {
        let def = self.tools.get(tool_name).ok_or_else(|| {
            ToolValidationError::UnknownTool {
                name: tool_name.to_string(),
            }
        })?;

        for param in def.required_params() {
            let present = arguments
                .as_object()
                .map(|obj| obj.contains_key(param))
                .unwrap_or(false);
            if !present {
                return Err(ToolValidationError::MissingRequiredParam {
                    tool: tool_name.to_string(),
                    param: param.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Returns `true` if there are no registered tools.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Returns names of all registered tools, sorted for determinism.
    pub fn tool_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.tools.keys().map(|s| s.as_str()).collect();
        names.sort_unstable();
        names
    }

    /// Renders a compact tool list block for injection into a system prompt.
    ///
    /// Returns `None` when the registry is empty.
    pub fn to_prompt_block(&self) -> Option<String> {
        if self.tools.is_empty() {
            return None;
        }
        let mut lines = vec!["Available tools (* = required param):".to_string()];
        let mut sorted: Vec<&ToolDefinition> = self.tools.values().collect();
        sorted.sort_by_key(|t| t.name.as_str());
        for tool in sorted {
            lines.push(tool.to_prompt_line());
        }
        Some(lines.join("\n"))
    }

    /// Load from a JSON array of `ToolDefinition`.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let defs: Vec<ToolDefinition> =
            serde_json::from_str(json).map_err(|e| format!("Invalid tools JSON: {e}"))?;
        let mut registry = Self::default();
        for def in defs {
            registry.register(def);
        }
        Ok(registry)
    }

    /// Serialize the full registry to a JSON array, sorted by name.
    pub fn to_json(&self) -> Result<String, String> {
        let mut tools: Vec<&ToolDefinition> = self.tools.values().collect();
        tools.sort_by_key(|t| t.name.as_str());
        serde_json::to_string(&tools).map_err(|e| format!("Serialize error: {e}"))
    }
}

// ============================================================================
// Agent Configuration
// ============================================================================

/// Agent behavior config (matches WIT agent-config)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum steps
    pub max_steps: u32,
    /// Verbose logging
    pub verbose: bool,
    /// Auto-execute tools
    pub auto_execute_tools: bool,
    /// Session timeout (seconds)
    pub session_timeout_secs: u32,
    /// Session ID
    pub session_id: String,
    /// Default context policy
    #[serde(default)]
    pub context_policy: ContextPolicy,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            verbose: false,
            auto_execute_tools: true,
            session_timeout_secs: 300,
            session_id: format!("session-{}", chrono::Utc::now().timestamp_millis()),
            context_policy: ContextPolicy {
                max_history_messages: 24,
                summarize_after_messages: 12,
                summary_max_chars: 1200,
                truncation_strategy: TruncationStrategy::KeepNewest,
            },
        }
    }
}

// ============================================================================
// Prompt Types
// ============================================================================

/// Prompt template variables
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptVariables {
    pub custom_instruction: Option<String>,
    pub language_guidance: Option<String>,
    pub tool_guidance: Option<String>,
    pub json_schema: Option<String>,
}

impl PromptVariables {
    /// Render template with variables
    pub fn render(&self, template: &str) -> String {
        let mut result = template.to_string();

        if let Some(val) = &self.custom_instruction {
            result = result.replace("{{custom_instruction}}", val);
        } else {
            result = result.replace("{{custom_instruction}}\n\n", "");
        }

        if let Some(val) = &self.language_guidance {
            result = result.replace("{{language_guidance}}", val);
        } else {
            result = result.replace("\n\n{{language_guidance}}", "");
        }

        if let Some(val) = &self.tool_guidance {
            result = result.replace("{{tool_guidance}}", val);
        } else {
            result = result.replace("\n\n{{tool_guidance}}", "");
        }

        if let Some(val) = &self.json_schema {
            result.push_str("\n\n");
            result.push_str(val);
        }

        result
    }
}

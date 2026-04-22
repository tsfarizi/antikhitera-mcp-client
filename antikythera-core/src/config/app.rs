use super::error::ConfigError;
use super::server::ServerConfig;
use super::tool::ToolConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// REST server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestServerConfig {
    /// Server bind address (e.g., "127.0.0.1:8080")
    #[serde(default = "default_bind")]
    pub bind: String,
    /// CORS allowed origins
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// API documentation servers
    #[serde(default)]
    pub docs: Vec<DocServerConfig>,
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

impl Default for RestServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            cors_origins: Vec::new(),
            docs: Vec::new(),
        }
    }
}

/// API documentation server entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocServerConfig {
    pub url: String,
    pub description: String,
}

/// Configurable prompts for agent behavior
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptsConfig {
    /// System prompt template with placeholders
    pub template: Option<String>,
    /// Guidance when tools are available
    pub tool_guidance: Option<String>,
    /// Guidance when no tools match the request
    pub fallback_guidance: Option<String>,
    /// Message sent to LLM when JSON parsing fails (retry prompt)
    pub json_retry_message: Option<String>,
    /// Instruction for tool result formatting
    pub tool_result_instruction: Option<String>,
    /// Base autonomous assistant rules and JSON constraints
    pub agent_instructions: Option<String>,
    /// Rules for late-binding UI hydration
    pub ui_instructions: Option<String>,
    /// Instructions for language detection and adherence
    pub language_instructions: Option<String>,
    /// User-facing error message for interaction limits
    pub agent_max_steps_error: Option<String>,
    /// Guidance when no tools are available or configured
    pub no_tools_guidance: Option<String>,
    /// Field names probed in fallback when the model returns an unknown action.
    /// Defaults to ["response", "content", "message"] when absent.
    pub fallback_response_keys: Option<Vec<String>>,
}

impl PromptsConfig {
    /// Default prompt template
    pub fn default_template() -> &'static str {
        "You are a helpful AI assistant.\n\n{{custom_instruction}}\n\n{{language_guidance}}\n\n{{tool_guidance}}"
    }

    /// Default tool guidance (English)
    pub fn default_tool_guidance() -> &'static str {
        "You have access to the following tools. Use them only when necessary to fulfill the user request:"
    }

    /// Default fallback guidance (English)
    pub fn default_fallback_guidance() -> &'static str {
        "If the request is outside the scope of available tools, apologize politely and explain your limitations."
    }

    /// Default JSON retry message (English)
    pub fn default_json_retry_message() -> &'static str {
        "System Error: Invalid JSON format returned. Please output ONLY the raw JSON object for the tool call or final response. Do not use Markdown blocks or explanations."
    }

    /// Default tool result instruction (English)
    pub fn default_tool_result_instruction() -> &'static str {
        "Tool execution complete. Process this result and respond with a VALID JSON object.\n\nIF THE TOOL RETURNED DATA (like a list of posts):\nYou MUST return a JSON object containing:\n1. \"message\": Your friendly text explanation.\n2. \"data\": The step ID reference.\n\nFormat for FINAL ANSWER:\n{\n  \"action\": \"final\",\n  \"response\": {\n    \"message\": \"Here is the information you requested...\",\n    \"data\": \"step_N\"\n  }\n}\n(Replace 'step_N' with the actual step ID, e.g., step_0)\n\nIF YOU NEED TO CALL ANOTHER TOOL:\n{\n  \"action\": \"call_tool\",\n  \"tool\": \"...\",\n  \"input\": {...}\n}\n\nCRITICAL:\n1. DO NOT return the raw data in the 'message' string.\n2. DO NOT summarize the data content in the 'message'.\n3. ALWAYS provide a 'message' so the user feels attended to."
    }

    /// Default agent instructions
    pub fn default_agent_instructions() -> &'static str {
        "You are an autonomous assistant that can call tools to solve user requests.\nAll responses must be valid JSON without commentary or code fences.\nWhen you need to invoke a single tool, respond with: {\"action\":\"call_tool\",\"tool\":\"tool_name\",\"input\":{...}}.\nWhen you need to invoke multiple tools simultaneously, respond with: {\"action\":\"call_tools\",\"tools\":[{\"name\":\"tool1\",\"input\":{...}}, {\"name\":\"tool2\",\"input\":{...}}]}.\nTo obtain the list of available tools, call the special tool: {\"action\":\"call_tool\",\"tool\":\"list_tools\"}.\nWhen you are ready to give the final answer to the user, respond with: {\"action\":\"final\",\"response\":{\"content\":\"...\", \"data\":\"step_N\"}} where 'step_N' refers to the index of a tool call result.\nIf your response includes data from tool calls, put the reference to the tool result in a 'data' field with the value 'step_N' where N is the step number.\nFor example: {\"action\":\"final\",\"response\":{\"content\":\"Here are the latest posts\",\"data\":\"step_0\"}}.\nIMPORTANT: Always return JSON for final responses, never plain text. If you want to include data from a tool call, reference it using the 'data' field with the appropriate step index.\nCRITICAL: Do not repeat or summarize the content of tool results in the 'content' field. Simply mention that the data exists and reference it using the 'data' field. The system will automatically embed the actual data from the tool result.\nABSOLUTELY CRITICAL: Your final response must be a JSON object with 'content' and 'data' fields. Do not return a string as the value of the 'response' field. The 'response' field must contain an object, not a string."
    }

    /// Default UI instructions
    pub fn default_ui_instructions() -> &'static str {
        "DATA FIELD REPLACEMENT:\nIf your response includes data from tool calls, put the reference to the tool result in a 'data' field with the value 'step_N' where N is the step number.\nThe system will automatically replace 'step_N' with the actual JSON data from the tool call result.\nFor example: {\"action\":\"final\",\"response\":{\"content\":\"Analysis complete\",\"data\":\"step_0\"}} where 'step_0' will be replaced with the actual data from the first tool call."
    }

    /// Default language instructions
    pub fn default_language_instructions() -> &'static str {
        "Detect the user's language automatically and answer using that same language unless they explicitly request another language.\nDo not call any translation-related tools; handle language understanding internally."
    }

    /// Default max steps error
    pub fn default_agent_max_steps_error() -> &'static str {
        "agent exceeded the maximum number of tool interactions"
    }

    /// Default no tools guidance
    pub fn default_no_tools_guidance() -> &'static str {
        "No additional tools are currently configured."
    }

    /// Get template with fallback to default
    pub fn template(&self) -> &str {
        self.template.as_deref().unwrap_or(Self::default_template())
    }

    /// Get tool guidance with fallback to default
    pub fn tool_guidance(&self) -> &str {
        self.tool_guidance
            .as_deref()
            .unwrap_or(Self::default_tool_guidance())
    }

    /// Get fallback guidance with fallback to default
    pub fn fallback_guidance(&self) -> &str {
        self.fallback_guidance
            .as_deref()
            .unwrap_or(Self::default_fallback_guidance())
    }

    /// Get JSON retry message with fallback to default
    pub fn json_retry_message(&self) -> &str {
        self.json_retry_message
            .as_deref()
            .unwrap_or(Self::default_json_retry_message())
    }

    /// Get tool result instruction with fallback to default
    pub fn tool_result_instruction(&self) -> &str {
        self.tool_result_instruction
            .as_deref()
            .unwrap_or(Self::default_tool_result_instruction())
    }

    /// Get agent instructions with fallback to default
    pub fn agent_instructions(&self) -> &str {
        self.agent_instructions
            .as_deref()
            .unwrap_or(Self::default_agent_instructions())
    }

    /// Get UI instructions with fallback to default
    pub fn ui_instructions(&self) -> &str {
        self.ui_instructions
            .as_deref()
            .unwrap_or(Self::default_ui_instructions())
    }

    /// Get language instructions with fallback to default
    pub fn language_instructions(&self) -> &str {
        self.language_instructions
            .as_deref()
            .unwrap_or(Self::default_language_instructions())
    }

    /// Get max steps error with fallback to default
    pub fn agent_max_steps_error(&self) -> &str {
        self.agent_max_steps_error
            .as_deref()
            .unwrap_or(Self::default_agent_max_steps_error())
    }

    /// Get no tools guidance with fallback to default
    pub fn no_tools_guidance(&self) -> &str {
        self.no_tools_guidance
            .as_deref()
            .unwrap_or(Self::default_no_tools_guidance())
    }

    /// Default fallback response key names
    pub fn default_fallback_response_keys() -> &'static [&'static str] {
        &["response", "content", "message"]
    }

    /// Field names probed when the model returns an unknown action
    pub fn fallback_response_keys(&self) -> Vec<&str> {
        match &self.fallback_response_keys {
            Some(keys) if !keys.is_empty() => keys.iter().map(String::as_str).collect(),
            _ => Self::default_fallback_response_keys().to_vec(),
        }
    }
}

/// Application runtime configuration for the MCP client.
///
/// This struct holds only the concerns that `antikythera-core` cares about:
/// MCP server connections, tool definitions, prompt customisation, and the
/// REST server bind settings.  Provider/model selection is a CLI concern and
/// is managed via [`super::postcard_config::PostcardAppConfig`] at the
/// CLI layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Preferred provider ID (opaque routing string, not a provider definition).
    pub default_provider: String,
    /// Preferred model name (opaque routing string).
    pub model: String,
    pub system_prompt: Option<String>,
    pub tools: Vec<ToolConfig>,
    pub servers: Vec<ServerConfig>,
    /// REST server settings (CORS, docs)
    pub rest_server: RestServerConfig,
    /// Configurable prompts for agent behavior
    pub prompts: PromptsConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_provider: "local".to_string(),
            model: "default".to_string(),
            system_prompt: None,
            tools: Vec::new(),
            servers: Vec::new(),
            rest_server: RestServerConfig::default(),
            prompts: PromptsConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from a file path (or default path if None)
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        super::loader::load_config(path)
    }

    /// Get the prompt template
    pub fn prompt_template(&self) -> &str {
        self.prompts.template()
    }

    /// Convert configuration to TOML string
    pub fn to_raw_toml(&self) -> String {
        super::serializer::to_raw_toml_string(self)
    }
}

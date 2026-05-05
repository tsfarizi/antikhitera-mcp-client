//! Builtin transport for in-process MCP tool implementations.
//!
//! This transport executes tool logic directly in-process without spawning
//! external MCP server processes. It is protocol-agnostic — the caller
//! provides tool definitions and handler functions at construction time.
//!
//! ## MCP Protocol Compliance
//!
//! - `tools/list` returns the full tool catalogue with `inputSchema`,
//!   `outputSchema`, `annotations`, and `execution` metadata per MCP spec.
//! - `tools/call` validates arguments against `inputSchema` before handler
//!   dispatch and validates results against `outputSchema` after execution.
//! - Validation errors use `isError: true` with LLM-actionable messages
//!   (not JSON-RPC protocol errors) so the model can self-correct.
//! - Tool names are validated against MCP naming rules at registration.
//! - All operations are logged via the core `TransportLogger`.
//!
//! ## Usage (pure Rust, no CLI dependency)
//!
//! ```ignore
//! use antikythera_core::application::tooling::transport::BuiltinTransport;
//! use antikythera_core::application::tooling::interface::ServerToolInfo;
//!
//! let tools = vec![ServerToolInfo { ... }];
//! let transport = BuiltinTransport::with_tools("my_server", tools)
//!     .with_handler("my_tool", |args| { ... })
//!     .with_instructions("Custom guidance text");
//! ```

use crate::application::tooling::error::ToolInvokeError;
use crate::application::tooling::interface::ServerToolInfo;
use crate::infrastructure::mcp::validate_tool_name;
use crate::logging::TransportLogger;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use super::McpTransport;

/// Builtin tool handler: receives the `arguments` JSON value, returns either
/// a result JSON value or an error message string.
pub type BuiltinToolFn = fn(arguments: &Value) -> Result<Value, String>;

/// In-process transport that bridges MCP protocol to Rust function handlers.
///
/// The transport itself carries zero tool logic — it is a pure protocol
/// adapter. Tool definitions, handlers, and instructions are all injected
/// by the caller.
#[derive(Debug, Clone)]
pub struct BuiltinTransport {
    inner: Arc<BuiltinTransportInner>,
}

struct BuiltinTransportInner {
    server_name: String,
    instructions: String,
    tools: Vec<ServerToolInfo>,
    handlers: HashMap<String, BuiltinToolFn>,
    tool_cache: AsyncMutex<HashMap<String, ServerToolInfo>>,
    input_schemas: HashMap<String, Value>,
    output_schemas: HashMap<String, Value>,
}

impl std::fmt::Debug for BuiltinTransportInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinTransportInner")
            .field("server_name", &self.server_name)
            .field("instructions", &self.instructions)
            .field("tools_count", &self.tools.len())
            .field("handler_count", &self.handlers.len())
            .field("tool_cache", &self.tool_cache)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl BuiltinTransport {
    /// Create a transport with the given server name and tool catalogue.
    ///
    /// Tools with invalid names (per MCP spec) are silently skipped with a
    /// log warning.
    pub fn with_tools(server_name: impl Into<String>, tools: Vec<ServerToolInfo>) -> Self {
        let server_name = server_name.into();
        let log = TransportLogger::new(&server_name);

        let valid_tools: Vec<ServerToolInfo> = tools
            .into_iter()
            .filter(|tool| {
                if let Err(reason) = validate_tool_name(&tool.name) {
                    log.warn(format!(
                        "skipping tool with invalid MCP name | tool={} reason={}",
                        tool.name, reason
                    ));
                    false
                } else {
                    true
                }
            })
            .collect();

        let instructions = Self::default_instructions(&valid_tools);

        let mut tool_cache = HashMap::new();
        let mut input_schemas = HashMap::new();
        let mut output_schemas = HashMap::new();
        for tool in &valid_tools {
            tool_cache.insert(tool.name.clone(), tool.clone());
            if let Some(ref schema) = tool.input_schema {
                input_schemas.insert(tool.name.clone(), schema.clone());
            }
            if let Some(ref schema) = tool.output_schema {
                output_schemas.insert(tool.name.clone(), schema.clone());
            }
        }

        log.info(format!(
            "initialised builtin transport | tools_count={}",
            valid_tools.len()
        ));

        Self {
            inner: Arc::new(BuiltinTransportInner {
                server_name,
                instructions,
                tools: valid_tools,
                handlers: HashMap::new(),
                tool_cache: AsyncMutex::new(tool_cache),
                input_schemas,
                output_schemas,
            }),
        }
    }

    /// Register a handler function for the named tool.
    ///
    /// Returns `self` for builder-style chaining.
    pub fn with_handler(mut self, tool_name: impl Into<String>, handler: BuiltinToolFn) -> Self {
        let tool_name = tool_name.into();
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.handlers.insert(tool_name, handler);
        } else {
            let mut new_handlers = HashMap::new();
            for (k, v) in &self.inner.handlers {
                new_handlers.insert(k.clone(), *v);
            }
            new_handlers.insert(tool_name, handler);
            self.inner = Arc::new(BuiltinTransportInner {
                server_name: self.inner.server_name.clone(),
                instructions: self.inner.instructions.clone(),
                tools: self.inner.tools.clone(),
                handlers: new_handlers,
                tool_cache: AsyncMutex::new(
                    self.inner
                        .tool_cache
                        .try_lock()
                        .map(|c| c.clone())
                        .unwrap_or_default(),
                ),
                input_schemas: self.inner.input_schemas.clone(),
                output_schemas: self.inner.output_schemas.clone(),
            });
        }
        self
    }

    /// Override the server instructions string returned to the agent.
    ///
    /// Returns `self` for builder-style chaining.
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        let instructions = instructions.into();
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.instructions = instructions;
        } else {
            self.inner = Arc::new(BuiltinTransportInner {
                server_name: self.inner.server_name.clone(),
                instructions,
                tools: self.inner.tools.clone(),
                handlers: self.inner.handlers.clone(),
                tool_cache: AsyncMutex::new(
                    self.inner
                        .tool_cache
                        .try_lock()
                        .map(|c| c.clone())
                        .unwrap_or_default(),
                ),
                input_schemas: self.inner.input_schemas.clone(),
                output_schemas: self.inner.output_schemas.clone(),
            });
        }
        self
    }

    fn default_instructions(tools: &[ServerToolInfo]) -> String {
        if tools.is_empty() {
            return "No built-in tools are currently configured.".to_string();
        }
        let names: Vec<String> = tools.iter().map(|t| format!("`{}`", t.name)).collect();
        format!(
            "Built-in utility tools available: {}. These tools run locally and are always available.",
            names.join(", ")
        )
    }
}

// ---------------------------------------------------------------------------
// Schema validators
// ---------------------------------------------------------------------------

/// Validate arguments against a JSON Schema `inputSchema`.
///
/// Supports the subset of JSON Schema used by MCP tool definitions:
/// - `type: "object"` at the top level
/// - `properties` with per-property `type` checks
/// - `required` array
/// - `additionalProperties: false`
///
/// Returns `Ok(())` on success, or an LLM-actionable error message.
pub fn validate_arguments(schema: &Value, arguments: &Value) -> Result<(), String> {
    // For tools with no parameters (empty schema or no schema), accept any valid object
    if schema.is_null() || schema.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        if arguments.is_null() || arguments.as_object().is_some() {
            return Ok(());
        }
        return Err("arguments must be a JSON object (even empty)".to_string());
    }

    let schema_obj = schema
        .as_object()
        .ok_or_else(|| "inputSchema must be a JSON object".to_string())?;

    // Top-level type must be "object" for tool inputs
    if let Some(schema_type) = schema_obj.get("type").and_then(Value::as_str)
        && schema_type != "object"
    {
        return Err(format!(
            "invalid tool schema: expected type 'object', got '{}'",
            schema_type
        ));
    }

    if !arguments.is_object() {
        return Err("arguments must be a JSON object matching the tool's input schema. Provide parameters as key-value pairs.".to_string());
    }

    let args_obj = arguments
        .as_object()
        .expect("arguments is not a JSON object — already checked by is_object()");

    // Check additionalProperties
    let allow_additional = schema_obj
        .get("additionalProperties")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    if !allow_additional
        && let Some(properties) = schema_obj.get("properties").and_then(Value::as_object)
    {
        for key in args_obj.keys() {
            if !properties.contains_key(key) {
                let allowed: Vec<&str> = properties.keys().map(|s| s.as_str()).collect();
                return Err(format!(
                    "unexpected parameter '{}'. Allowed parameters: [{}]",
                    key,
                    allowed.join(", ")
                ));
            }
        }
    }

    // Check required params
    if let Some(required) = schema_obj.get("required").and_then(Value::as_array) {
        for req in required {
            if let Some(name) = req.as_str()
                && !args_obj.contains_key(name)
            {
                return Err(format!(
                    "missing required parameter '{}'. Please provide a value for this parameter.",
                    name
                ));
            }
        }
    }

    // Validate property types
    if let Some(properties) = schema_obj.get("properties").and_then(Value::as_object) {
        for (prop_name, prop_schema) in properties {
            if let Some(value) = args_obj.get(prop_name) {
                let expected_type = prop_schema
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("string");
                if !value_matches_type(value, expected_type) {
                    return Err(format!(
                        "parameter '{}' must be of type '{}', but received '{}'. Please correct the value.",
                        prop_name,
                        expected_type,
                        json_type_name(value)
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Validate a tool result against its `outputSchema`.
///
/// Returns `Ok(())` on success, or logs a warning and returns `Ok(())`
/// anyway — output validation is advisory (MCP spec says clients SHOULD
/// validate, not MUST).
pub fn validate_output(schema: &Value, result: &Value, log: &TransportLogger, tool_name: &str) {
    if schema.is_null() || schema.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        return;
    }

    let Some(schema_obj) = schema.as_object() else {
        log.warn(format!(
            "outputSchema is not a valid JSON object | tool={}",
            tool_name
        ));
        return;
    };

    if !result.is_object() {
        log.warn(format!(
            "output is not an object, expected per outputSchema | tool={}",
            tool_name
        ));
        return;
    }

    let result_obj = result
        .as_object()
        .expect("result is not a JSON object — already checked by is_object()");

    if let Some(required) = schema_obj.get("required").and_then(Value::as_array) {
        for req in required {
            if let Some(name) = req.as_str()
                && !result_obj.contains_key(name)
            {
                log.warn(format!(
                    "output missing required field '{}' per outputSchema | tool={}",
                    name, tool_name
                ));
            }
        }
    }

    if let Some(properties) = schema_obj.get("properties").and_then(Value::as_object) {
        for (prop_name, prop_schema) in properties {
            if let Some(value) = result_obj.get(prop_name)
                && let Some(expected_type) = prop_schema.get("type").and_then(Value::as_str)
                && !value_matches_type(value, expected_type)
            {
                log.warn(format!(
                    "output field '{}' type mismatch: expected '{}', got '{}' | tool={}",
                    prop_name,
                    expected_type,
                    json_type_name(value),
                    tool_name
                ));
            }
        }
    }
}

fn value_matches_type(value: &Value, expected_type: &str) -> bool {
    match expected_type {
        "string" => value.is_string(),
        "number" | "integer" => value.is_number(),
        "boolean" => value.is_boolean(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "null" => value.is_null(),
        _ => true, // unknown type — be permissive
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ---------------------------------------------------------------------------
// McpTransport implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl McpTransport for BuiltinTransport {
    async fn connect(&self) -> Result<(), ToolInvokeError> {
        Ok(())
    }

    async fn send_request(&self, method: &str, params: Value) -> Result<Value, ToolInvokeError> {
        let log = TransportLogger::new(&self.inner.server_name);

        match method {
            "tools/list" => {
                log.debug("handling tools/list request");
                let tools_json: Vec<Value> = self
                    .inner
                    .tools
                    .iter()
                    .map(|tool| {
                        json!({
                            "name": tool.name,
                            "title": tool.title,
                            "description": tool.description,
                            "icons": tool.icons,
                            "inputSchema": tool.input_schema,
                            "outputSchema": tool.output_schema,
                            "annotations": tool.annotations,
                            "execution": tool.execution,
                        })
                    })
                    .collect();
                Ok(json!({ "tools": tools_json }))
            }
            "tools/call" => {
                let tool_name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
                    log.warn("tools/call received without tool name");
                    ToolInvokeError::Transport {
                        server: self.inner.server_name.clone(),
                        message: "missing tool name in tools/call params".to_string(),
                    }
                })?;

                let arguments = params.get("arguments").unwrap_or(&Value::Null).clone();

                log.info(format!("dispatching tool call | tool={}", tool_name));

                // --- 1. Validate input against inputSchema ---
                if let Some(input_schema) = self.inner.input_schemas.get(tool_name)
                    && let Err(validation_err) = validate_arguments(input_schema, &arguments)
                {
                    log.warn(format!(
                        "input validation failed | tool={} error={}",
                        tool_name, validation_err
                    ));
                    return Ok(json!({
                        "content": [{ "type": "text", "text": validation_err }],
                        "isError": true
                    }));
                }

                // --- 2. Look up handler ---
                let handler = self.inner.handlers.get(tool_name).ok_or_else(|| {
                    log.warn(format!(
                        "tool not found in builtin registry | tool={}",
                        tool_name
                    ));
                    ToolInvokeError::Rpc {
                        server: self.inner.server_name.clone(),
                        code: -32602,
                        message: format!(
                            "unknown tool: '{}'. Use tools/list to discover available tools.",
                            tool_name
                        ),
                    }
                })?;

                // --- 3. Execute handler ---
                match handler(&arguments) {
                    Ok(result) => {
                        // --- 4. Validate output against outputSchema ---
                        if let Some(output_schema) = self.inner.output_schemas.get(tool_name) {
                            validate_output(output_schema, &result, &log, tool_name);
                        }

                        let result_text =
                            serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
                        log.info(format!("tool execution succeeded | tool={}", tool_name));
                        Ok(json!({
                            "content": [{ "type": "text", "text": result_text }],
                            "structuredContent": result,
                            "isError": false
                        }))
                    }
                    Err(err) => {
                        log.warn(format!(
                            "tool execution failed | tool={} error={}",
                            tool_name, err
                        ));
                        Ok(json!({
                            "content": [{ "type": "text", "text": err }],
                            "isError": true
                        }))
                    }
                }
            }
            _ => {
                log.warn(format!("unsupported method requested | method={}", method));
                Err(ToolInvokeError::Rpc {
                    server: self.inner.server_name.clone(),
                    code: -32601,
                    message: format!("method not supported: '{}'", method),
                })
            }
        }
    }

    async fn send_notification(
        &self,
        _method: &str,
        _params: Value,
    ) -> Result<(), ToolInvokeError> {
        Ok(())
    }

    async fn call_tool(&self, tool: &str, arguments: Value) -> Result<Value, ToolInvokeError> {
        let params = json!({
            "name": tool,
            "arguments": match arguments {
                Value::Null => Value::Object(Default::default()),
                other => other,
            }
        });
        self.send_request("tools/call", params).await
    }

    async fn instructions(&self) -> Option<String> {
        Some(self.inner.instructions.clone())
    }

    async fn tool_metadata(&self, tool: &str) -> Option<ServerToolInfo> {
        self.inner.tool_cache.lock().await.get(tool).cloned()
    }

    async fn list_tools(&self) -> Vec<ServerToolInfo> {
        self.inner
            .tool_cache
            .lock()
            .await
            .values()
            .cloned()
            .collect()
    }

    fn server_name(&self) -> &str {
        &self.inner.server_name
    }

    async fn is_connected(&self) -> bool {
        true
    }

    async fn disconnect(&self) {
        let log = TransportLogger::new(&self.inner.server_name);
        log.debug("builtin transport disconnected");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tooling::interface::{TaskSupport, ToolAnnotations, ToolExecution};

    // -- validation tests ---------------------------------------------------

    #[test]
    fn test_validate_arguments_missing_required() {
        let schema = json!({
            "type": "object",
            "properties": {
                "city": { "type": "string" }
            },
            "required": ["city"]
        });
        let args = json!({});
        let err = validate_arguments(&schema, &args).unwrap_err();
        assert!(err.contains("missing required parameter 'city'"));
    }

    #[test]
    fn test_validate_arguments_type_mismatch() {
        let schema = json!({
            "type": "object",
            "properties": {
                "count": { "type": "number" }
            }
        });
        let args = json!({"count": "not_a_number"});
        let err = validate_arguments(&schema, &args).unwrap_err();
        assert!(err.contains("must be of type 'number'"));
    }

    #[test]
    fn test_validate_arguments_additional_properties_rejected() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "additionalProperties": false
        });
        let args = json!({"name": "test", "extra": true});
        let err = validate_arguments(&schema, &args).unwrap_err();
        assert!(err.contains("unexpected parameter 'extra'"));
    }

    #[test]
    fn test_validate_arguments_valid() {
        let schema = json!({
            "type": "object",
            "properties": {
                "city": { "type": "string" },
                "days": { "type": "number" }
            },
            "required": ["city"]
        });
        let args = json!({"city": "Jakarta", "days": 7});
        assert!(validate_arguments(&schema, &args).is_ok());
    }

    #[test]
    fn test_validate_arguments_no_schema_accepts_any() {
        assert!(validate_arguments(&Value::Null, &json!({"any": "thing"})).is_ok());
    }

    #[test]
    fn test_validate_arguments_empty_schema_accepts_empty_object() {
        let schema = json!({
            "type": "object",
            "additionalProperties": false
        });
        assert!(validate_arguments(&schema, &json!({})).is_ok());
        assert!(validate_arguments(&schema, &json!(null)).is_err());
    }

    // -- transport tests ----------------------------------------------------

    fn make_echo_tool() -> (Vec<ServerToolInfo>, BuiltinToolFn) {
        let tool = ServerToolInfo {
            name: "echo".to_string(),
            title: Some("Echo Tool".to_string()),
            description: Some("Returns the input unchanged".to_string()),
            icons: None,
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string", "description": "Text to echo" }
                },
                "required": ["message"],
                "additionalProperties": false
            })),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "echo": { "type": "string" }
                },
                "required": ["echo"]
            })),
            annotations: Some(ToolAnnotations {
                audience: Some(vec!["assistant".to_string()]),
                priority: Some(0.5),
                last_modified: None,
            }),
            execution: Some(ToolExecution {
                task_support: Some(TaskSupport::Forbidden),
            }),
        };

        let handler: BuiltinToolFn = |args: &Value| {
            let msg = args
                .get("message")
                .and_then(Value::as_str)
                .ok_or("missing 'message' parameter")?;
            Ok(json!({ "echo": msg }))
        };

        (vec![tool], handler)
    }

    #[tokio::test]
    async fn test_list_tools() {
        let (tools, handler) = make_echo_tool();
        let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
        let listed = transport.list_tools().await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "echo");
    }

    #[tokio::test]
    async fn test_call_tool_success() {
        let (tools, handler) = make_echo_tool();
        let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
        let result = transport
            .call_tool("echo", json!({"message": "hello"}))
            .await
            .unwrap();
        let is_error = result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        assert!(!is_error);
        let sc = result.get("structuredContent").unwrap();
        assert_eq!(sc.get("echo").and_then(Value::as_str), Some("hello"));
    }

    #[tokio::test]
    async fn test_call_tool_input_validation_missing_required() {
        let (tools, handler) = make_echo_tool();
        let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
        // Missing required "message"
        let result = transport.call_tool("echo", json!({})).await.unwrap();
        let is_error = result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        assert!(is_error, "missing required param must return isError: true");
        let text = result
            .get("content")
            .and_then(|c| c.get(0))
            .and_then(|b| b.get("text"))
            .and_then(Value::as_str)
            .unwrap();
        assert!(text.contains("missing required parameter 'message'"));
    }

    #[tokio::test]
    async fn test_call_tool_input_validation_extra_param() {
        let (tools, handler) = make_echo_tool();
        let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
        // Extra unknown parameter
        let result = transport
            .call_tool("echo", json!({"message": "hi", "unknown": 42}))
            .await
            .unwrap();
        let is_error = result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        assert!(is_error, "extra param must return isError: true");
    }

    #[tokio::test]
    async fn test_call_tool_input_validation_type_mismatch() {
        let (tools, handler) = make_echo_tool();
        let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
        // "message" should be string, not number
        let result = transport
            .call_tool("echo", json!({"message": 123}))
            .await
            .unwrap();
        let is_error = result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        assert!(is_error, "type mismatch must return isError: true");
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let (tools, handler) = make_echo_tool();
        let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
        let result = transport.call_tool("nonexistent", json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_custom_instructions() {
        let (tools, handler) = make_echo_tool();
        let transport = BuiltinTransport::with_tools("test", tools)
            .with_handler("echo", handler)
            .with_instructions("Custom guidance");
        assert_eq!(
            transport.instructions().await,
            Some("Custom guidance".to_string())
        );
    }

    #[tokio::test]
    async fn test_invalid_tool_name_skipped() {
        let bad_tool = ServerToolInfo {
            name: "bad name with spaces".to_string(),
            title: None,
            description: None,
            icons: None,
            input_schema: None,
            output_schema: None,
            annotations: None,
            execution: None,
        };
        let transport = BuiltinTransport::with_tools("test", vec![bad_tool]);
        assert!(transport.list_tools().await.is_empty());
    }

    #[tokio::test]
    async fn test_tool_metadata() {
        let (tools, handler) = make_echo_tool();
        let transport = BuiltinTransport::with_tools("test", tools).with_handler("echo", handler);
        let meta = transport.tool_metadata("echo").await.unwrap();
        assert_eq!(meta.name, "echo");
        assert!(meta.input_schema.is_some());
        assert!(meta.output_schema.is_some());
    }

    #[tokio::test]
    async fn test_empty_tools() {
        let transport = BuiltinTransport::with_tools("test", vec![]);
        assert!(transport.list_tools().await.is_empty());
        assert_eq!(
            transport.instructions().await,
            Some("No built-in tools are currently configured.".to_string())
        );
    }
}

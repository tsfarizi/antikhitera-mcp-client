use super::directive::AgentDirective;
use super::errors::AgentError;
use super::models::{AgentOptions, AgentOutcome, AgentStep};
use super::runtime::ToolRuntime;
use crate::application::client::{ChatRequest, McpClient};
use crate::model::ModelProvider;
use serde_json::{Value, json};
use std::sync::Arc;
use sysinfo::System;
use tracing::{debug, info, warn};

/// Maximum retry attempts for JSON parsing failures
const MAX_JSON_RETRIES: u8 = 3;

pub struct Agent<P: ModelProvider> {
    client: Arc<McpClient<P>>,
    runtime: ToolRuntime,
}

impl<P: ModelProvider> Agent<P> {
    pub fn new(client: Arc<McpClient<P>>) -> Self {
        let tools = client.tools().to_vec();
        let bridge = client.server_bridge();
        Self {
            client,
            runtime: ToolRuntime::new(tools, bridge),
        }
    }

    pub async fn run(
        &self,
        prompt: String,
        mut options: AgentOptions,
    ) -> Result<AgentOutcome, AgentError> {
        info!("Agent run started");
        let mut session_id = options.session_id.clone();
        let mut steps = Vec::new();
        let mut logs = Vec::new();

        let context = self.runtime.build_context(Some(&prompt)).await;
        let instructions = self
            .runtime
            .compose_system_instructions(&context, self.client.prompts());
        let system_prompt = match options.system_prompt.take() {
            Some(existing) if !existing.trim().is_empty() => {
                format!("{existing}\n\n{instructions}")
            }
            _ => instructions,
        };

        let prompt_preview = McpClient::<P>::summarise(&prompt);
        let mut next_prompt = self.runtime.initial_user_prompt(prompt, &context);
        logs.push(format!("Initial agent request: {prompt_preview}"));

        let effective_provider = self.client.default_provider().to_string();
        let effective_model = self.client.default_model().to_string();
        logs.push(format!(
            "Active provider: '{effective_provider}' | Model: '{effective_model}'"
        ));

        let mut remaining_steps = options.max_steps;
        let mut system_prompt_to_send = Some(system_prompt);
        let mut system = System::new();
        let mut first_call = true;
        let initial_attachments = std::mem::take(&mut options.attachments);

        loop {
            system.refresh_cpu_all();
            system.refresh_memory();
            let rss_mb = system.used_memory() / 1024 / 1024;
            let cpu = system.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
                / system.cpus().len().max(1) as f32;
            debug!(
                rss_mb = rss_mb,
                cpu_usage = cpu,
                "Agent resource utilization"
            );

            debug!(
                session = session_id.as_deref(),
                remaining_steps, "Submitting agent turn to model provider"
            );
            let request = ChatRequest {
                prompt: next_prompt.clone(),
                attachments: if first_call {
                    initial_attachments.clone()
                } else {
                    Vec::new()
                },
                system_prompt: if first_call {
                    system_prompt_to_send.take()
                } else {
                    None
                },
                session_id: session_id.clone(),
                raw_mode: false,
                bypass_template: true, // Agent composes its own complete system prompt
                force_json: true,
            };

            let result = self.client.chat(request).await?;
            logs.extend(result.logs.clone());
            session_id = Some(result.session_id.clone());
            first_call = false;

            // Parse agent action with retry logic for malformed JSON
            let directive = self
                .parse_with_retry(&result.content, &mut logs, &session_id)
                .await?;

            match directive {
                AgentDirective::Final { response } => {
                    info!(
                        session_id = result.session_id.as_str(),
                        "Agent returned final response"
                    );
                    return Ok(AgentOutcome {
                        logs,
                        session_id: result.session_id,
                        response,
                        steps,
                    });
                }
                AgentDirective::CallTool { tool, input } => {
                    if remaining_steps == 0 {
                        warn!("Agent exceeded max tool interactions");
                        return Err(AgentError::InvalidResponse(
                            self.client.prompts().agent_max_steps_error().into(),
                        ));
                    }
                    remaining_steps -= 1;
                    info!(tool = %tool, "Agent requested tool execution");
                    let execution = self.runtime.execute(&tool, input).await?;
                    logs.push(format!(
                        "Tool '{}' executed (success: {})",
                        execution.tool, execution.success
                    ));
                    if let Some(message) = execution.message.as_deref() {
                        logs.push(format!(
                            "Tool message: {}",
                            McpClient::<P>::summarise(message)
                        ));
                    }

                    steps.push(AgentStep {
                        tool: execution.tool.clone(),
                        input: execution.input.clone(),
                        success: execution.success,
                        output: execution.output.clone(),
                        message: execution.message.clone(),
                    });

                    // Use configurable tool result instruction
                    // Use configurable tool result instruction
                    let tool_result_instruction = self.client.prompts().tool_result_instruction();
                    next_prompt = json!({
                        "tool_result": {
                            "tool": execution.tool,
                            "input": execution.input,
                            "success": execution.success,
                            "output": execution.output,
                            "message": execution.message,
                        },
                        "instruction": tool_result_instruction,
                    })
                    .to_string();
                }
                AgentDirective::CallTools(tools) => {
                    if remaining_steps == 0 {
                        warn!("Agent exceeded max tool interactions");
                        return Err(AgentError::InvalidResponse(
                            self.client.prompts().agent_max_steps_error().into(),
                        ));
                    }
                    remaining_steps -= 1;
                    info!(
                        count = tools.len(),
                        "Agent requested parallel tool execution"
                    );

                    let executions = self.runtime.clone().execute_parallel(tools).await?;
                    let mut aggregated_results = Vec::new();

                    for exec_result in executions {
                        match exec_result {
                            Ok(execution) => {
                                logs.push(format!(
                                    "Tool '{}' executed (success: {})",
                                    execution.tool, execution.success
                                ));
                                if let Some(message) = execution.message.as_deref() {
                                    logs.push(format!(
                                        "Tool message: {}",
                                        McpClient::<P>::summarise(message)
                                    ));
                                }

                                steps.push(AgentStep {
                                    tool: execution.tool.clone(),
                                    input: execution.input.clone(),
                                    success: execution.success,
                                    output: execution.output.clone(),
                                    message: execution.message.clone(),
                                });

                                aggregated_results.push(json!({
                                    "tool": execution.tool,
                                    "input": execution.input,
                                    "success": execution.success,
                                    "output": execution.output,
                                    "message": execution.message,
                                }));
                            }
                            Err(e) => {
                                warn!("One of the parallel tools failed: {}", e);
                                logs.push(format!("Parallel tool failure: {}", e));
                            }
                        }
                    }

                    let tool_result_instruction = self.client.prompts().tool_result_instruction();
                    next_prompt = json!({
                        "tool_results": aggregated_results,
                        "instruction": tool_result_instruction,
                    })
                    .to_string();
                }
            }
        }
    }

    /// Run agent and return response with embedded tool results.
    pub async fn run_ui_layout(
        &self,
        prompt: String,
        options: AgentOptions,
    ) -> Result<(AgentOutcome, serde_json::Value), AgentError> {
        // 1. Run the agent loop
        let outcome = self.run(prompt, options).await?;

        // 2. Process the response to embed tool results by replacing IDs with actual data
        let processed_response =
            self.embed_tool_results_sync(outcome.response.clone(), &outcome.steps);

        // 3. If the processed response is a string (meaning the LLM didn't follow JSON format),
        // wrap it in a proper structure with content field
        let final_response = match processed_response {
            Value::String(s) => {
                // If the LLM returned a plain string, wrap it in a content field
                json!({"content": s})
            }
            _ => processed_response,
        };

        Ok((outcome, final_response))
    }

    /// Embed tool results into the response by replacing IDs with actual data from tool steps.
    fn embed_tool_results_sync(&self, response: Value, steps: &[AgentStep]) -> Value {
        match response {
            Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (key, value) in obj {
                    let processed_value = self.embed_tool_results_sync(value, steps);
                    new_obj.insert(key, processed_value);
                }
                Value::Object(new_obj)
            }
            Value::Array(arr) => {
                // Process top-level arrays
                let new_arr: Vec<Value> = arr
                    .into_iter()
                    .map(|item| self.embed_tool_results_sync(item, steps))
                    .collect();
                Value::Array(new_arr)
            }
            Value::String(s) => {
                // 1. Check if the entire string is just a step reference (e.g., "step_0")
                // In this case, we replace the whole string with the actual data object/array
                if (s.starts_with("step_") || s.starts_with("result_")) && !s.contains(' ') {
                    if let Some(step_idx) = s
                        .strip_prefix("step_")
                        .or_else(|| s.strip_prefix("result_"))
                    {
                        if let Ok(idx) = step_idx.parse::<usize>() {
                            // Try 0-based index first, then 1-based index (if idx > 0)
                            if let Some(step) = steps.get(idx) {
                                return self.extract_result_data(&step.output);
                            } else if idx > 0 {
                                if let Some(step) = steps.get(idx - 1) {
                                    return self.extract_result_data(&step.output);
                                }
                            }
                        }
                    }
                }

                // 2. Check for step references embedded within a larger string
                // We'll use a simple approach here: look for "step_N" or "result_N" patterns
                // and replace them with stringified JSON if they exist.
                let mut result_str = s.clone();
                let mut modified = false;

                // Iterate in reverse to avoid partial matches (e.g., "step_1" matching "step_10")
                // We check both 0-based and 1-based logic by iterating up to steps.len() + 1
                for i in (0..=steps.len()).rev() {
                    let step_pattern = format!("step_{}", i);
                    let result_pattern = format!("result_{}", i);

                    if result_str.contains(&step_pattern) || result_str.contains(&result_pattern) {
                        // Resolve the index: if i is out of bounds, try i-1
                        let step_to_use = if i < steps.len() {
                            Some(&steps[i])
                        } else if i > 0 && (i - 1) < steps.len() {
                            Some(&steps[i - 1])
                        } else {
                            None
                        };

                        if let Some(step) = step_to_use {
                            let data = self.extract_result_data(&step.output);
                            let replacement = match &data {
                                Value::String(inner_s) => inner_s.clone(),
                                _ => serde_json::to_string(&data)
                                    .unwrap_or_else(|_| "null".to_string()),
                            };

                            result_str = result_str.replace(&step_pattern, &replacement);
                            result_str = result_str.replace(&result_pattern, &replacement);
                            modified = true;
                        }
                    }
                }

                if modified {
                    Value::String(result_str)
                } else {
                    Value::String(s)
                }
            }
            _ => response, // Other values (numbers, booleans, null) remain unchanged
        }
    }

    /// Extract just the result data from a tool output, filtering out JSON-RPC wrapper if present.
    fn extract_result_data(&self, output: &Value) -> Value {
        // If the output looks like a JSON-RPC response with a "result" field, extract just that
        if let Some(obj) = output.as_object() {
            if let Some(result) = obj.get("result") {
                return result.clone();
            }
            // If it has other JSON-RPC fields like "jsonrpc", "id", "error", extract just the meaningful data
            if obj.contains_key("jsonrpc") || obj.contains_key("id") {
                // Return the result field if present, otherwise return the whole object minus JSON-RPC fields
                if let Some(result) = obj.get("result") {
                    return result.clone();
                } else {
                    // If there's no result field but it's a JSON-RPC object, return null
                    // Or return the original if it has other meaningful data
                    let filtered_obj: serde_json::Map<String, Value> = obj
                        .iter()
                        .filter(|(k, _)| !["jsonrpc", "id", "error"].contains(&k.as_str()))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();

                    if filtered_obj.is_empty() {
                        // If no non-RPC fields remain, return the original
                        return output.clone();
                    } else {
                        return Value::Object(filtered_obj);
                    }
                }
            }

            // Handle MCP content format: {"content": [{"type": "text", "text": "..."}]}
            if let Some(content_arr) = obj.get("content").and_then(|c| c.as_array()) {
                if content_arr.len() == 1 {
                    if let Some(block) = content_arr[0].as_object() {
                        if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                // Try to parse the text as JSON
                                if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                                    return parsed;
                                }
                                // If not JSON, return the text as a string
                                return Value::String(text.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Otherwise, return the output as-is
        output.clone()
    }

    /// Parse agent action with retry logic for malformed JSON
    async fn parse_with_retry(
        &self,
        content: &str,
        logs: &mut Vec<String>,
        session_id: &Option<String>,
    ) -> Result<AgentDirective, AgentError> {
        let mut retry_count = 0u8;
        let mut current_content = content.to_string();

        loop {
            match self.runtime.parse_agent_action(&current_content) {
                Ok(directive) => return Ok(directive),
                Err(e) if retry_count < MAX_JSON_RETRIES => {
                    retry_count += 1;
                    warn!(
                        attempt = retry_count,
                        max_attempts = MAX_JSON_RETRIES,
                        error = %e,
                        "JSON parse failed, requesting correction from model"
                    );
                    logs.push(format!(
                        "JSON parse retry attempt {}/{}: {}",
                        retry_count, MAX_JSON_RETRIES, e
                    ));

                    // Get retry message from config
                    let retry_message = format!(
                        "{}\n\nError details: {}",
                        self.client.prompts().json_retry_message(),
                        e
                    );

                    // Send correction request to model
                    let retry_request = ChatRequest {
                        prompt: retry_message,
                        attachments: Vec::new(),
                        system_prompt: None,
                        session_id: session_id.clone(),
                        raw_mode: false,
                        bypass_template: true, // Agent composes its own complete system prompt
                        force_json: true,
                    };

                    match self.client.chat(retry_request).await {
                        Ok(retry_result) => {
                            logs.extend(retry_result.logs.clone());
                            current_content = retry_result.content;
                        }
                        Err(chat_err) => {
                            warn!(error = %chat_err, "Retry chat request failed");
                            return Err(AgentError::InvalidResponse(format!(
                                "Failed to get correction after JSON parse error: {}",
                                chat_err
                            )));
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        attempts = retry_count,
                        "JSON parse failed after max retries"
                    );
                    return Err(AgentError::InvalidResponse(format!(
                        "Invalid JSON after {} retry attempts: {}",
                        MAX_JSON_RETRIES, e
                    )));
                }
            }
        }
    }
}

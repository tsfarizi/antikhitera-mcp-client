/// JSON parse-with-retry logic shared by all agent runners.
///
/// When the model returns malformed JSON, the runtime sends a correction
/// request back and retries up to [`MAX_JSON_RETRIES`] times before giving up.
/// Moving this into `ToolRuntime` removes the identical copy that previously
/// lived in both `runner.rs` and `fsm_runner.rs`.
use super::{AgentDirective, AgentError, ToolRuntime};
use crate::application::client::{ChatRequest, McpClient};
use crate::application::model_provider::ModelProvider;
use crate::logging::AgentLogger;
use std::sync::Arc;

/// Maximum retry attempts for JSON parsing failures.
pub(crate) const MAX_JSON_RETRIES: u8 = 3;

impl ToolRuntime {
    /// Parse agent action from `content`, retrying up to [`MAX_JSON_RETRIES`]
    /// times by sending a correction request through `client` when the model
    /// returns malformed JSON.
    ///
    /// # Arguments
    ///
    /// * `content`    — Raw model response text to parse.
    /// * `client`     — The [`McpClient`] used to send correction requests.
    /// * `logs`       — Mutable log accumulator; retry attempts are appended.
    /// * `session_id` — Current session identifier forwarded to correction requests.
    pub(crate) async fn parse_with_retry<P: ModelProvider>(
        &self,
        content: &str,
        client: &Arc<McpClient<P>>,
        logs: &mut Vec<String>,
        session_id: &Option<String>,
    ) -> Result<AgentDirective, AgentError> {
        let log = AgentLogger::new(
            session_id
                .as_deref()
                .unwrap_or(&crate::logging::get_active_session()),
        );
        let mut retry_count = 0u8;
        let mut current_content = content.to_string();

        loop {
            match self.parse_agent_action(&current_content) {
                Ok(directive) => return Ok(directive),
                Err(e) if retry_count < MAX_JSON_RETRIES => {
                    retry_count += 1;
                    log.warn(format!(
                        "JSON parse failed, requesting correction from model | attempt={} max_attempts={} error={}",
                        retry_count, MAX_JSON_RETRIES, e
                    ));
                    logs.push(format!(
                        "JSON parse retry attempt {}/{}: {}",
                        retry_count, MAX_JSON_RETRIES, e
                    ));

                    let retry_message = format!(
                        "{}\n\nError details: {}",
                        client.prompts().json_retry_message(),
                        e
                    );

                    let retry_request = ChatRequest {
                        prompt: retry_message,
                        attachments: Vec::new(),
                        system_prompt: None,
                        session_id: session_id.clone(),
                        raw_mode: false,
                        bypass_template: true,
                        force_json: true,
                    };

                    match client.chat(retry_request).await {
                        Ok(retry_result) => {
                            logs.extend(retry_result.logs.clone());
                            current_content = retry_result.content;
                        }
                        Err(chat_err) => {
                            log.warn(format!("Retry chat request failed | error={}", chat_err));
                            return Err(AgentError::InvalidResponse(format!(
                                "Failed to get correction after JSON parse error: {}",
                                chat_err
                            )));
                        }
                    }
                }
                Err(e) => {
                    log.warn(format!(
                        "JSON parse failed after max retries | attempts={}",
                        retry_count
                    ));
                    return Err(AgentError::InvalidResponse(format!(
                        "Invalid JSON after {} retry attempts: {}",
                        MAX_JSON_RETRIES, e
                    )));
                }
            }
        }
    }
}

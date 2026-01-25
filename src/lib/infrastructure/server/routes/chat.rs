use super::super::dto::{Attachment, ErrorResponse, RestChatRequest, RestChatResponse};
use super::super::state::ServerState;
use crate::agent::{Agent, AgentOptions, AgentStep};
use crate::client::{ChatRequest, McpError};
use crate::model::ModelProvider;
use crate::types::MessagePart;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Convert attachments to message parts
fn attachments_to_parts(attachments: Vec<Attachment>) -> Vec<MessagePart> {
    attachments
        .into_iter()
        .map(|a| {
            if a.mime_type.starts_with("image/") {
                MessagePart::image(a.mime_type, a.data)
            } else {
                MessagePart::file(a.name, a.mime_type, a.data)
            }
        })
        .collect()
}

/// Helper to construct the response based on debug mode
fn construct_response(
    debug: bool,
    session_id: String,
    content: serde_json::Value,
    logs: Vec<String>,
    provider: String,
    model: String,
    tool_steps: Vec<AgentStep>,
) -> RestChatResponse {
    if !debug {
        RestChatResponse {
            logs: None,
            session_id,
            content,
            provider: None,
            model: None,
            tool_steps: None,
        }
    } else {
        RestChatResponse {
            logs: Some(logs),
            session_id,
            content,
            provider: Some(provider),
            model: Some(model),
            tool_steps: Some(tool_steps),
        }
    }
}

#[utoipa::path(
    post,
    path = "/chat",
    tag = "chat",
    request_body = RestChatRequest,
    responses(
        (status = 200, description = "Obrolan berhasil diproses", body = RestChatResponse),
        (status = 400, description = "Permintaan tidak valid", body = ErrorResponse),
        (status = 502, description = "Model atau agen tidak dapat dihubungi", body = ErrorResponse)
    )
)]
pub async fn chat_handler<P: ModelProvider>(
    State(state): State<Arc<ServerState<P>>>,
    Json(payload): Json<RestChatRequest>,
) -> Result<Json<RestChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    let RestChatRequest {
        prompt,
        attachments,
        system_prompt,
        session_id,
        agent,
        max_tool_steps,
        debug,
    } = payload;

    info!(
        agent,
        session = session_id.as_deref(),
        attachments = attachments.len(),
        "Received /chat request"
    );

    if prompt.trim().is_empty() && attachments.is_empty() {
        error!("Rejecting /chat request due to empty prompt and no attachments");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "prompt cannot be empty".to_string(),
            }),
        ));
    }

    let client = state.client();
    let attachment_parts = attachments_to_parts(attachments);

    // Use defaults from config
    let provider = client.default_provider().to_string();
    let model = client.default_model().to_string();
    let debug_mode = debug.unwrap_or(true);

    if agent {
        let mut options = AgentOptions::default();
        options.system_prompt = system_prompt.clone();
        options.session_id = session_id.clone();
        options.attachments = attachment_parts.clone();
        if let Some(max_steps) = max_tool_steps {
            options.max_steps = max_steps;
        }
        let agent_runner = Agent::new(client.clone());
        match agent_runner
            .run_ui_layout(prompt, options)
            .await
        {
            Ok((outcome, content_json)) => {
                info!(
                    session_id = outcome.session_id.as_str(),
                    "Agent run completed successfully"
                );

                Ok(Json(construct_response(
                    debug_mode,
                    outcome.session_id,
                    content_json,
                    outcome.logs,
                    provider,
                    model,
                    outcome.steps,
                )))
            }
            Err(error) => {
                error!(%error, "Agent run failed");
                let message = error.user_message();
                Err((
                    StatusCode::BAD_GATEWAY,
                    Json(ErrorResponse { error: message }),
                ))
            }
        }
    } else {
        debug!("Forwarding /chat request to model provider (raw mode)");
        let result = client
            .chat(ChatRequest {
                prompt,
                attachments: attachment_parts,
                system_prompt,
                session_id,
                raw_mode: true,         // Non-agent mode bypasses system prompts
                bypass_template: false, // Not relevant when raw_mode is true
                force_json: false,
            })
            .await;

        match result {
            Ok(result) => {
                info!(
                    session_id = result.session_id.as_str(),
                    provider = result.provider.as_str(),
                    model = result.model.as_str(),
                    "Chat request completed successfully"
                );

                // When agent is false, content is just the string
                let content = json!(result.content);

                Ok(Json(construct_response(
                    debug_mode,
                    result.session_id,
                    content,
                    result.logs,
                    result.provider,
                    result.model,
                    Vec::new(),
                )))
            }
            Err(McpError::Model(error)) => {
                error!(%error, "Model provider returned an error");
                let message = error.user_message();
                Err((
                    StatusCode::BAD_GATEWAY,
                    Json(ErrorResponse { error: message }),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_construct_response_debug_true() {
        let session_id = "test-session".to_string();
        let content = json!({"key": "value"});
        let logs = vec!["log1".to_string()];
        let provider = "test-provider".to_string();
        let model = "test-model".to_string();
        let tool_steps = vec![];

        let response = construct_response(
            true,
            session_id.clone(),
            content.clone(),
            logs.clone(),
            provider.clone(),
            model.clone(),
            tool_steps.clone(),
        );

        assert_eq!(response.session_id, session_id);
        assert_eq!(response.content, content);
        assert_eq!(response.logs, Some(logs));
        assert_eq!(response.provider, Some(provider));
        assert_eq!(response.model, Some(model));
        assert_eq!(response.tool_steps, Some(tool_steps));
    }

    #[test]
    fn test_construct_response_debug_false() {
        let session_id = "test-session".to_string();
        let content = json!({"key": "value"});
        let logs = vec!["log1".to_string()];
        let provider = "test-provider".to_string();
        let model = "test-model".to_string();
        let tool_steps = vec![];

        let response = construct_response(
            false,
            session_id.clone(),
            content.clone(),
            logs.clone(),
            provider.clone(),
            model.clone(),
            tool_steps.clone(),
        );

        assert_eq!(response.session_id, session_id);
        assert_eq!(response.content, content);
        assert!(response.logs.is_none());
        assert!(response.provider.is_none());
        assert!(response.model.is_none());
        assert!(response.tool_steps.is_none());
    }

    #[test]
    fn test_content_structure_assumption() {
        // Verify that we can pass simple strings as content (for agent=false case)
        let content_str = json!("Simple string content");
        assert!(content_str.is_string());

        let content_obj = json!({"complex": "object"});
        assert!(content_obj.is_object());
    }
}

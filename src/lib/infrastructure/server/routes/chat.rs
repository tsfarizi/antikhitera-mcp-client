use super::super::dto::{Attachment, ErrorResponse, RestChatRequest, RestChatResponse};
use super::super::state::ServerState;
use crate::agent::{Agent, AgentOptions};
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

    if agent {
        let mut options = AgentOptions::default();
        options.system_prompt = system_prompt.clone();
        options.session_id = session_id.clone();
        options.attachments = attachment_parts.clone();
        if let Some(max_steps) = max_tool_steps {
            options.max_steps = max_steps;
        }
        let agent = Agent::new(client.clone());
        match agent
            .run_ui_layout(prompt, options)
            .await
        {
            Ok((outcome, content_json)) => {
                info!(
                    session_id = outcome.session_id.as_str(),
                    "Agent run completed successfully"
                );

                Ok(Json(RestChatResponse {
                    logs: outcome.logs,
                    session_id: outcome.session_id,
                    content: content_json,
                    provider,
                    model,
                    tool_steps: outcome.steps,
                }))
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
                Ok(Json(RestChatResponse {
                    logs: result.logs,
                    session_id: result.session_id,
                    content: json!({
                        "type": "text",
                        "props": {
                            "content": result.content
                        }
                    }),
                    provider: result.provider,
                    model: result.model,
                    tool_steps: Vec::new(),
                }))
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

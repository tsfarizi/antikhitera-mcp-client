use super::super::dto::{ErrorResponse, RestChatRequest, RestChatResponse};
use super::super::state::ServerState;
use crate::agent::{Agent, AgentOptions};
use crate::client::{ChatRequest, McpError};
use crate::model::ModelProvider;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use std::sync::Arc;
use tracing::{debug, error, info};

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
    info!(
        agent = payload.agent,
        session = payload.session_id.as_deref(),
        "Received /chat request"
    );

    if payload.prompt.trim().is_empty() {
        error!("Rejecting /chat request due to empty prompt");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "prompt cannot be empty".to_string(),
            }),
        ));
    }

    let client = state.client();

    if payload.agent {
        let mut options = AgentOptions::default();
        options.model = payload.model;
        options.system_prompt = payload.system_prompt;
        options.session_id = payload.session_id;
        if let Some(max_steps) = payload.max_tool_steps {
            options.max_steps = max_steps;
        }
        let agent = Agent::new(client.clone());
        match agent.run(payload.prompt, options).await {
            Ok(outcome) => {
                info!(
                    session_id = outcome.session_id.as_str(),
                    "Agent run completed successfully"
                );
                Ok(Json(RestChatResponse {
                    session_id: outcome.session_id,
                    content: outcome.response,
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
        debug!("Forwarding /chat request to model provider");
        let result = client
            .chat(ChatRequest {
                prompt: payload.prompt,
                model: payload.model,
                system_prompt: payload.system_prompt,
                session_id: payload.session_id,
            })
            .await;

        match result {
            Ok(result) => {
                info!(
                    session_id = result.session_id.as_str(),
                    "Chat request completed successfully"
                );
                Ok(Json(RestChatResponse {
                    session_id: result.session_id,
                    content: result.content,
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

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
    let RestChatRequest {
        prompt,
        provider,
        model,
        system_prompt,
        session_id,
        agent,
        max_tool_steps,
    } = payload;

    info!(
        agent,
        session = session_id.as_deref(),
        provider = provider.as_deref(),
        model = model.as_deref(),
        "Received /chat request"
    );

    if prompt.trim().is_empty() {
        error!("Rejecting /chat request due to empty prompt");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "prompt cannot be empty".to_string(),
            }),
        ));
    }

    let client = state.client();

    if agent {
        let mut options = AgentOptions::default();
        let provider_for_agent = provider.clone();
        let model_for_agent = model.clone();
        let resolved_provider = provider_for_agent
            .clone()
            .unwrap_or_else(|| client.default_provider().to_string());
        let resolved_model = model_for_agent
            .clone()
            .unwrap_or_else(|| client.default_model().to_string());
        options.provider = provider_for_agent;
        options.model = model_for_agent;
        options.system_prompt = system_prompt.clone();
        options.session_id = session_id.clone();
        if let Some(max_steps) = max_tool_steps {
            options.max_steps = max_steps;
        }
        let agent = Agent::new(client.clone());
        match agent.run(prompt, options).await {
            Ok(outcome) => {
                info!(
                    session_id = outcome.session_id.as_str(),
                    "Agent run completed successfully"
                );
                Ok(Json(RestChatResponse {
                    logs: outcome.logs,
                    session_id: outcome.session_id,
                    content: outcome.response,
                    provider: resolved_provider,
                    model: resolved_model,
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
                prompt,
                provider,
                model,
                system_prompt,
                session_id,
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
                    content: result.content,
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

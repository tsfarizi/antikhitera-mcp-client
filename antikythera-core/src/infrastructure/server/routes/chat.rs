use super::super::dto::{Attachment, ErrorResponse, RestChatRequest, RestChatResponse};
use super::super::state::ServerState;
use crate::application::services::chat::ChatService;
use crate::infrastructure::model::ModelProvider;
use crate::domain::types::MessagePart;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use std::sync::Arc;
use tracing::{error, info};

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
    let debug_mode = debug.unwrap_or(false);

    let chat_service = ChatService::new(client);

    match chat_service
        .process_request(
            prompt,
            attachment_parts,
            system_prompt,
            session_id,
            agent,
            max_tool_steps,
            debug_mode,
        )
        .await
    {
        Ok(outcome) => Ok(Json(RestChatResponse {
            logs: outcome.logs,
            session_id: outcome.session_id,
            content: outcome.content,
            provider: outcome.provider,
            model: outcome.model,
            tool_steps: outcome.tool_steps,
        })),
        Err(err_msg) => {
            // Note: Currently mapping all service errors to BAD_GATEWAY (502)
            // In a more granular implementation, ChatService could return typed errors
            Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse { error: err_msg }),
            ))
        }
    }
}

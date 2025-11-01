use super::super::dto::ToolInventoryResponse;
use super::super::state::ServerState;
use crate::agent::ToolRuntime;
use crate::model::ModelProvider;
use axum::Json;
use axum::extract::State;
use std::sync::Arc;
use tracing::debug;

#[utoipa::path(
    get,
    path = "/tools",
    tag = "tools",
    responses(
        (status = 200, description = "Daftar tools tersedia", body = ToolInventoryResponse)
    )
)]
pub async fn tools_handler<P: ModelProvider>(
    State(state): State<Arc<ServerState<P>>>,
) -> Json<ToolInventoryResponse> {
    let client = state.client();
    let runtime = ToolRuntime::new(client.tools().to_vec(), client.server_bridge());
    let context = runtime.build_context().await;
    debug!(
        tool_count = context.tools.len(),
        server_count = context.servers.len(),
        "Serving /tools request"
    );
    Json(ToolInventoryResponse::from(context))
}

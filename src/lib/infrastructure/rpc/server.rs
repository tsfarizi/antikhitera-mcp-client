use crate::agent::{Agent, AgentOptions};
use crate::model::ModelProvider;
use crate::rpc::types::{RpcRequest, RpcResponse};
use crate::server::ServerState;
use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{debug, error, info};

pub(crate) async fn handle_rpc<P: ModelProvider>(
    State(state): State<Arc<ServerState<P>>>,
    Json(request): Json<RpcRequest>,
) -> Json<RpcResponse> {
    debug!(method = %request.method, "Received JSON-RPC request");

    if request.jsonrpc != "2.0" {
        return Json(RpcResponse::invalid_request(
            "Unsupported jsonrpc version (expected 2.0)",
        ));
    }

    let response = match request.method.as_str() {
        "mcp.session.create" => handle_session_create(&state, request.id.clone()).await,
        "mcp.session.list" => handle_session_list(&state, request.id.clone()).await,
        "mcp.tools.list" => handle_tool_list(&state, request.id.clone()).await,
        "mcp.chat.message" => handle_chat_message(&state, &request).await,
        other => {
            error!(method = other, "Unknown JSON-RPC method");
            RpcResponse::method_not_found(request.id.clone(), other)
        }
    };

    Json(response)
}

async fn handle_session_create<P: ModelProvider>(
    _state: &Arc<ServerState<P>>,
    id: Option<Value>,
) -> RpcResponse {
    let session_id = uuid::Uuid::new_v4().to_string();
    RpcResponse::success(
        id,
        json!({
            "session_id": session_id,
        }),
    )
}

async fn handle_session_list<P: ModelProvider>(
    _state: &Arc<ServerState<P>>,
    id: Option<Value>,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "sessions": []
        }),
    )
}

async fn handle_tool_list<P: ModelProvider>(
    state: &Arc<ServerState<P>>,
    id: Option<Value>,
) -> RpcResponse {
    let client = state.client();
    let tools = client.tools();
    RpcResponse::success(
        id,
        json!({
            "tools": tools.iter().map(|tool| json!({
                "name": tool.name,
                "description": tool.description,
            })).collect::<Vec<_>>()
        }),
    )
}

async fn handle_chat_message<P: ModelProvider>(
    state: &Arc<ServerState<P>>,
    request: &RpcRequest,
) -> RpcResponse {
    let Some(Value::Object(params)) = &request.params else {
        return RpcResponse::error(
            request.id.clone(),
            -32602,
            "params must be an object with prompt",
        );
    };

    let prompt = match params.get("prompt") {
        Some(Value::String(value)) if !value.trim().is_empty() => value.clone(),
        _ => {
            return RpcResponse::error(
                request.id.clone(),
                -32602,
                "params.prompt must be a non-empty string",
            );
        }
    };

    let session_id = params
        .get("session_id")
        .and_then(|value| value.as_str().map(ToOwned::to_owned));

    let provider = params
        .get("provider")
        .and_then(|value| value.as_str().map(ToOwned::to_owned));
    let model = params
        .get("model")
        .and_then(|value| value.as_str().map(ToOwned::to_owned));
    let system_prompt = params
        .get("system_prompt")
        .and_then(|value| value.as_str().map(ToOwned::to_owned));

    let client = state.client();
    let resolved_provider = provider
        .clone()
        .unwrap_or_else(|| client.default_provider().to_string());
    let resolved_model = model
        .clone()
        .unwrap_or_else(|| client.default_model().to_string());

    let mut options = AgentOptions::default();
    options.provider = provider;
    options.model = model;
    options.system_prompt = system_prompt;
    options.session_id = session_id.clone();
    let agent = Agent::new(client.clone());
    info!(
        ?session_id,
        provider = resolved_provider.as_str(),
        model = resolved_model.as_str(),
        "Processing chat message via JSON-RPC"
    );

    match agent.run(prompt, options).await {
        Ok(outcome) => {
            let result = json!({
                "session_id": outcome.session_id,
                "content": outcome.response,
                "provider": resolved_provider,
                "model": resolved_model,
                "tool_steps": outcome.steps,
                "logs": outcome.logs,
            });
            RpcResponse::success(request.id.clone(), result)
        }
        Err(error) => {
            error!(%error, "Agent run failed via JSON-RPC");
            RpcResponse::error(request.id.clone(), -32000, error.user_message())
        }
    }
}

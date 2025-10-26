use crate::agent::{Agent, AgentOptions, AgentStep};
use crate::client::{ChatRequest, McpClient, McpError};
use crate::config::{AppConfig, CONFIG_PATH, ConfigError, DEFAULT_PROMPT_TEMPLATE, ToolConfig};
use crate::model::ModelProvider;
use crate::rpc::server::handle_rpc;
use axum::extract::State;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("failed to bind HTTP listener on {addr}: {source}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("HTTP server error: {0}")]
    Serve(#[from] std::io::Error),
}

pub(crate) struct ServerState<P: ModelProvider> {
    client: Arc<McpClient<P>>,
}

impl<P: ModelProvider> ServerState<P> {
    pub(crate) fn new(client: Arc<McpClient<P>>) -> Self {
        Self { client }
    }

    pub(crate) fn client(&self) -> Arc<McpClient<P>> {
        Arc::clone(&self.client)
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        chat_handler,
        tools_handler,
        config_get_handler,
        config_put_handler
    ),
    components(
        schemas(
            RestChatRequest,
            RestChatResponse,
            ErrorResponse,
            ToolListResponse,
            ConfigResponse,
            ConfigUpdateRequest,
            AgentStep,
            ToolConfig
        )
    ),
    tags(
        (name = "chat", description = "Interaksi warga dengan LLM atau agen"),
        (name = "tools", description = "Daftar tool MCP yang tersedia"),
        (name = "config", description = "Manajemen konfigurasi klien MCP")
    )
)]
struct ApiDoc;

pub async fn serve<P>(client: Arc<McpClient<P>>, addr: SocketAddr) -> Result<(), ServerError>
where
    P: ModelProvider + 'static,
{
    let api = ApiDoc::openapi();
    info!(%addr, "Binding REST server");

    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://127.0.0.1:5173"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::OPTIONS])
        .allow_headers(Any);

    let state = Arc::new(ServerState::new(client));
    let app = Router::new()
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", api))
        .route("/chat", post(chat_handler::<P>))
        .route("/tools", get(tools_handler::<P>))
        .route(
            "/config-file",
            get(config_get_handler::<P>).put(config_put_handler::<P>),
        )
        .route("/rpc", post(handle_rpc::<P>))
        .layer(cors)
        .with_state(state);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|source| ServerError::Bind { addr, source })?;
    info!(%addr, "REST server ready to accept connections");

    axum::serve(listener, app.into_make_service())
        .await
        .map_err(ServerError::Serve)
}

#[derive(Debug, Deserialize, ToSchema)]
struct RestChatRequest {
    prompt: String,
    model: Option<String>,
    system_prompt: Option<String>,
    session_id: Option<String>,
    #[serde(default)]
    agent: bool,
    #[serde(default)]
    max_tool_steps: Option<usize>,
}

#[derive(Debug, Serialize, ToSchema)]
struct RestChatResponse {
    session_id: String,
    content: String,
    tool_steps: Vec<AgentStep>,
}

#[derive(Debug, Serialize, ToSchema)]
struct ErrorResponse {
    error: String,
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
async fn chat_handler<P: ModelProvider>(
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

#[derive(Debug, Serialize, ToSchema)]
struct ToolListResponse {
    tools: Vec<ToolConfig>,
}

#[utoipa::path(
    get,
    path = "/tools",
    tag = "tools",
    responses(
        (status = 200, description = "Daftar tools tersedia", body = ToolListResponse)
    )
)]
async fn tools_handler<P: ModelProvider>(
    State(state): State<Arc<ServerState<P>>>,
) -> Json<ToolListResponse> {
    let client = state.client();
    let count = client.tools().len();
    debug!(tool_count = count, "Serving /tools request");
    let tools = client.tools().iter().cloned().collect();
    Json(ToolListResponse { tools })
}

#[derive(Debug, Serialize, ToSchema)]
struct ConfigResponse {
    model: String,
    system_prompt: Option<String>,
    prompt_template: String,
    tools: Vec<ToolConfig>,
    raw: String,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ConfigUpdateRequest {
    model: String,
    system_prompt: Option<String>,
    prompt_template: String,
}

#[utoipa::path(
    get,
    path = "/config-file",
    tag = "config",
    responses(
        (status = 200, description = "Konfigurasi MCP saat ini", body = ConfigResponse),
        (status = 500, description = "Gagal memuat konfigurasi", body = ErrorResponse)
    )
)]
async fn config_get_handler<P: ModelProvider>(
    State(_state): State<Arc<ServerState<P>>>,
) -> Result<Json<ConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    let path = Path::new(CONFIG_PATH);
    let config = match AppConfig::load(Some(path)) {
        Ok(config) => config,
        Err(ConfigError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
            AppConfig::default()
        }
        Err(error) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("failed to load config: {error}"),
                }),
            ));
        }
    };

    let prompt_template = config
        .prompt_template
        .clone()
        .unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string());

    let raw = std::fs::read_to_string(path).unwrap_or_else(|_| {
        render_config_raw(
            &config.model,
            config.system_prompt.as_deref(),
            &prompt_template,
            &config.tools,
        )
    });

    Ok(Json(ConfigResponse {
        model: config.model,
        system_prompt: config.system_prompt,
        prompt_template,
        tools: config.tools,
        raw,
    }))
}

#[utoipa::path(
    put,
    path = "/config-file",
    tag = "config",
    request_body = ConfigUpdateRequest,
    responses(
        (status = 200, description = "Konfigurasi MCP diperbarui", body = ConfigResponse),
        (status = 500, description = "Gagal menyimpan konfigurasi", body = ErrorResponse)
    )
)]
async fn config_put_handler<P: ModelProvider>(
    State(_state): State<Arc<ServerState<P>>>,
    Json(payload): Json<ConfigUpdateRequest>,
) -> Result<Json<ConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    let path = Path::new(CONFIG_PATH);
    let mut config = AppConfig::load(Some(path)).unwrap_or_else(|_| AppConfig::default());
    config.model = payload.model;
    config.system_prompt = payload.system_prompt;
    config.prompt_template = Some(payload.prompt_template.clone());

    if let Some(parent) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("failed to prepare config directory: {error}"),
                }),
            ));
        }
    }

    let prompt_template = config
        .prompt_template
        .clone()
        .unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string());
    let raw = render_config_raw(
        &config.model,
        config.system_prompt.as_deref(),
        &prompt_template,
        &config.tools,
    );

    if let Err(error) = std::fs::write(path, &raw) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("failed to write config: {error}"),
            }),
        ));
    }

    info!(path = %path.display(), "Configuration updated via REST");

    Ok(Json(ConfigResponse {
        model: config.model,
        system_prompt: config.system_prompt,
        prompt_template,
        tools: config.tools,
        raw,
    }))
}

fn render_config_raw(
    model: &str,
    system_prompt: Option<&str>,
    prompt_template: &str,
    tools: &[ToolConfig],
) -> String {
    let mut raw = format!("model = \"{}\"\n\n", model);
    if let Some(system_prompt) = system_prompt {
        raw.push_str(&format!(
            "system_prompt = \"{}\"\n\n",
            system_prompt.replace('"', "\\\""),
        ));
    }
    raw.push_str("prompt_template = \"\"\"\n");
    raw.push_str(prompt_template);
    if !prompt_template.ends_with('\n') {
        raw.push('\n');
    }
    raw.push_str("\"\"\"\n");
    if !tools.is_empty() {
        raw.push_str("\n");
        raw.push_str("tools = [\n");
        for tool in tools {
            match &tool.description {
                Some(desc) => raw.push_str(&format!(
                    "    {{ name = \"{}\", description = \"{}\" }},\n",
                    tool.name.replace('"', "\\\""),
                    desc.replace('"', "\\\""),
                )),
                None => raw.push_str(&format!("    \"{}\",\n", tool.name.replace('"', "\\\""),)),
            }
        }
        raw.push_str("]\n");
    }
    raw
}

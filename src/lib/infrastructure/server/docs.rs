use super::dto::{
    ConfigResponse, ConfigUpdateRequest, ErrorResponse, ReloadResponse, RestChatRequest,
    RestChatResponse, ToolInventoryResponse,
};
use super::routes;
use crate::agent::AgentStep;
use crate::config::{DocServerConfig, ToolConfig};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::chat::chat_handler,
        routes::tools::tools_handler,
        routes::config::config_get_handler,
        routes::config::config_put_handler,
        routes::config::config_reload_handler
    ),
    components(
        schemas(
            RestChatRequest,
            RestChatResponse,
            ErrorResponse,
            ToolInventoryResponse,
            ConfigResponse,
            ConfigUpdateRequest,
            ReloadResponse,
            AgentStep,
            ToolConfig
        )
    ),
    tags(
        (name = "chat", description = "LLM and agent interactions"),
        (name = "tools", description = "Available MCP tools"),
        (name = "config", description = "Client configuration management")
    )
)]
pub(super) struct ApiDoc;

impl ApiDoc {
    /// Create OpenAPI spec with servers from config
    pub fn with_servers(doc_servers: &[DocServerConfig]) -> utoipa::openapi::OpenApi {
        let mut api = Self::openapi();
        if !doc_servers.is_empty() {
            api.servers = Some(
                doc_servers
                    .iter()
                    .map(|s| {
                        utoipa::openapi::ServerBuilder::new()
                            .url(&s.url)
                            .description(Some(&s.description))
                            .build()
                    })
                    .collect(),
            );
        }
        api
    }
}

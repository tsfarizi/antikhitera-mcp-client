use super::dto::{
    ConfigResponse, ConfigUpdateRequest, ErrorResponse, RestChatRequest, RestChatResponse,
    ToolInventoryResponse,
};
use super::routes;
use crate::agent::AgentStep;
use crate::config::ToolConfig;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    servers(
        (url = "https://5w4m7wvp-8080.asse.devtunnels.ms", description = "Staging server"),
    ),
    paths(
        routes::chat::chat_handler,
        routes::tools::tools_handler,
        routes::config::config_get_handler,
        routes::config::config_put_handler
    ),
    components(
        schemas(
            RestChatRequest,
            RestChatResponse,
            ErrorResponse,
            ToolInventoryResponse,
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
pub(super) struct ApiDoc;

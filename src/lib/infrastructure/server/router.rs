use super::docs::ApiDoc;
use super::error::ServerError;
use super::routes;
use super::state::ServerState;
use crate::client::McpClient;
use crate::model::ModelProvider;
use crate::rpc::server::handle_rpc;
use axum::Router;
use axum::http::{HeaderValue, Method};
use axum::routing::{get, post};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub(super) async fn serve<P>(client: Arc<McpClient<P>>, addr: SocketAddr) -> Result<(), ServerError>
where
    P: ModelProvider + 'static,
{
    let api = ApiDoc::openapi();
    info!(%addr, "Binding REST server");

    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://127.0.0.1:5173"),
            HeaderValue::from_static("https://tsfarizi.github.io"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::OPTIONS])
        .allow_headers(Any);

    let state = Arc::new(ServerState::new(client));
    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", api))
        .route("/chat", post(routes::chat::chat_handler::<P>))
        .route("/tools", get(routes::tools::tools_handler::<P>))
        .route(
            "/config-file",
            get(routes::config::config_get_handler::<P>)
                .put(routes::config::config_put_handler::<P>),
        )
        .route("/reload", post(routes::config::config_reload_handler::<P>))
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

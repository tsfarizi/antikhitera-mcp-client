use super::super::dto::{ConfigResponse, ConfigUpdateRequest, ErrorResponse, ReloadResponse};
use crate::config::{AppConfig, CONFIG_PATH};
use crate::model::ModelProvider;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

use super::super::state::ServerState;

#[utoipa::path(
    get,
    path = "/config-file",
    tag = "config",
    responses(
        (status = 200, description = "Konfigurasi MCP saat ini", body = ConfigResponse),
        (status = 500, description = "Gagal memuat konfigurasi", body = ErrorResponse)
    )
)]
pub async fn config_get_handler<P: ModelProvider>(
    State(_state): State<Arc<ServerState<P>>>,
) -> Result<Json<ConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    let path = Path::new(CONFIG_PATH);
    let config = match AppConfig::load(Some(path)) {
        Ok(config) => config,
        Err(error) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("failed to load config: {error}"),
                }),
            ));
        }
    };

    let prompt_template = config.prompt_template().to_string();
    let raw = std::fs::read_to_string(path).unwrap_or_else(|_| config.to_raw_toml());

    Ok(Json(ConfigResponse {
        model: config.model,
        default_provider: config.default_provider,
        system_prompt: config.system_prompt,
        prompt_template,
        tools: config.tools,
        providers: config.providers,
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
pub async fn config_put_handler<P: ModelProvider>(
    State(_state): State<Arc<ServerState<P>>>,
    Json(payload): Json<ConfigUpdateRequest>,
) -> Result<Json<ConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    let path = Path::new(CONFIG_PATH);
    let mut config = match AppConfig::load(Some(path)) {
        Ok(c) => c,
        Err(error) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("failed to load existing config: {error}"),
                }),
            ));
        }
    };
    config.model = payload.model;
    config.default_provider = payload.default_provider;
    config.system_prompt = payload.system_prompt;
    config.prompts.template = Some(payload.prompt_template.clone());

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

    let raw = config.to_raw_toml();
    let prompt_template = config.prompt_template().to_string();

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
        default_provider: config.default_provider,
        system_prompt: config.system_prompt,
        prompt_template,
        tools: config.tools,
        providers: config.providers,
        raw,
    }))
}

#[utoipa::path(
    post,
    path = "/reload",
    tag = "config",
    responses(
        (status = 200, description = "Konfigurasi dimuat ulang dari file", body = ReloadResponse),
        (status = 500, description = "Gagal memuat konfigurasi", body = ErrorResponse)
    )
)]
pub async fn config_reload_handler<P: ModelProvider>(
    State(_state): State<Arc<ServerState<P>>>,
) -> Result<Json<ReloadResponse>, (StatusCode, Json<ErrorResponse>)> {
    let path = Path::new(CONFIG_PATH);

    info!(path = %path.display(), "Reloading configuration from file");

    match AppConfig::load(Some(path)) {
        Ok(config) => {
            let raw = std::fs::read_to_string(path).unwrap_or_else(|_| config.to_raw_toml());
            let prompt_template = config.prompt_template().to_string();

            info!("Configuration reloaded successfully");

            Ok(Json(ReloadResponse {
                success: true,
                message: "Konfigurasi berhasil dimuat ulang. Restart aplikasi untuk menerapkan perubahan.".to_string(),
                config: Some(ConfigResponse {
                    model: config.model,
                    default_provider: config.default_provider,
                    system_prompt: config.system_prompt,
                    prompt_template,
                    tools: config.tools,
                    providers: config.providers,
                    raw,
                }),
            }))
        }
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Gagal memuat konfigurasi: {error}"),
            }),
        )),
    }
}

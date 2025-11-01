use super::super::dto::{ConfigResponse, ConfigUpdateRequest, ErrorResponse};
use crate::config::{
    AppConfig, CONFIG_PATH, ConfigError, DEFAULT_PROMPT_TEMPLATE, ModelProviderConfig,
    ProviderKind, ToolConfig,
};
use crate::model::ModelProvider;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use std::io;
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
            &config.default_provider,
            &config.model,
            config.system_prompt.as_deref(),
            &prompt_template,
            &config.tools,
            &config.providers,
        )
    });

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
    let mut config = AppConfig::load(Some(path)).unwrap_or_else(|_| AppConfig::default());
    config.model = payload.model;
    config.default_provider = payload.default_provider;
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
        &config.default_provider,
        &config.model,
        config.system_prompt.as_deref(),
        &prompt_template,
        &config.tools,
        &config.providers,
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
        default_provider: config.default_provider,
        system_prompt: config.system_prompt,
        prompt_template,
        tools: config.tools,
        providers: config.providers,
        raw,
    }))
}

fn render_config_raw(
    default_provider: &str,
    model: &str,
    system_prompt: Option<&str>,
    prompt_template: &str,
    tools: &[ToolConfig],
    providers: &[ModelProviderConfig],
) -> String {
    let escape = |value: &str| value.replace('"', "\\\"");
    let mut raw = format!(
        "default_provider = \"{}\"\nmodel = \"{}\"\n\n",
        escape(default_provider),
        escape(model)
    );

    if let Some(system_prompt) = system_prompt {
        raw.push_str(&format!(
            "system_prompt = \"{}\"\n\n",
            escape(system_prompt),
        ));
    }

    raw.push_str("prompt_template = \"\"\"\n");
    raw.push_str(prompt_template);
    if !prompt_template.ends_with('\n') {
        raw.push('\n');
    }
    raw.push_str("\"\"\"\n");

    if !providers.is_empty() {
        raw.push('\n');
        for provider in providers {
            raw.push_str("[[providers]]\n");
            raw.push_str(&format!("id = \"{}\"\n", escape(&provider.id)));
            let kind = match provider.kind {
                ProviderKind::Ollama => "ollama",
                ProviderKind::Gemini => "gemini",
            };
            raw.push_str(&format!("type = \"{}\"\n", kind));
            raw.push_str(&format!("endpoint = \"{}\"\n", escape(&provider.endpoint)));
            if let Some(api_key) = &provider.api_key {
                raw.push_str(&format!("api_key = \"{}\"\n", escape(api_key)));
            }
            raw.push_str("models = [\n");
            for model_info in &provider.models {
                match &model_info.display_name {
                    Some(label) => raw.push_str(&format!(
                        "    {{ name = \"{}\", display_name = \"{}\" }},\n",
                        escape(&model_info.name),
                        escape(label),
                    )),
                    None => raw.push_str(&format!(
                        "    {{ name = \"{}\" }},\n",
                        escape(&model_info.name),
                    )),
                }
            }
            raw.push_str("]\n\n");
        }
    }

    if !tools.is_empty() {
        raw.push_str("tools = [\n");
        for tool in tools {
            match &tool.description {
                Some(desc) => raw.push_str(&format!(
                    "    {{ name = \"{}\", description = \"{}\" }},\n",
                    escape(&tool.name),
                    escape(desc),
                )),
                None => raw.push_str(&format!("    \"{}\",\n", escape(&tool.name))),
            }
        }
        raw.push_str("]\n");
    }

    raw
}

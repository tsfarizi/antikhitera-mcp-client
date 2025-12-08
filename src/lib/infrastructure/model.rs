use crate::config::ModelProviderConfig;
use crate::constants::DEFAULT_GEMINI_API_PATH;
use crate::types::{ChatMessage, MessageRole};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub provider: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub message: ChatMessage,
    pub session_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("provider '{provider}' is not configured")]
    ProviderNotFound { provider: String },
    #[error("model '{model}' is not available for provider '{provider}'")]
    ModelNotFound { provider: String, model: String },
    #[error("provider '{provider}' requires an API key")]
    MissingApiKey { provider: String },
    #[error("network error calling provider '{provider}': {source}")]
    Network {
        provider: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("provider '{provider}' returned invalid response: {reason}")]
    InvalidResponse { provider: String, reason: String },
    #[error("unsupported provider type '{provider_type}' for provider '{provider}'")]
    UnsupportedProviderType {
        provider: String,
        provider_type: String,
    },
}

impl ModelError {
    pub fn provider_not_found(provider: impl Into<String>) -> Self {
        Self::ProviderNotFound {
            provider: provider.into(),
        }
    }

    pub fn model_not_found(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self::ModelNotFound {
            provider: provider.into(),
            model: model.into(),
        }
    }

    pub fn missing_api_key(provider: impl Into<String>) -> Self {
        Self::MissingApiKey {
            provider: provider.into(),
        }
    }

    pub fn network(provider: impl Into<String>, source: reqwest::Error) -> Self {
        Self::Network {
            provider: provider.into(),
            source,
        }
    }

    pub fn invalid_response(provider: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidResponse {
            provider: provider.into(),
            reason: reason.into(),
        }
    }

    pub fn unsupported_provider_type(
        provider: impl Into<String>,
        provider_type: impl Into<String>,
    ) -> Self {
        Self::UnsupportedProviderType {
            provider: provider.into(),
            provider_type: provider_type.into(),
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            ModelError::ProviderNotFound { provider } => format!(
                "Penyedia model '{provider}' tidak ditemukan. Periksa pengaturan client.toml."
            ),
            ModelError::ModelNotFound { provider, model } => format!(
                "Model '{model}' tidak tersedia pada penyedia '{provider}'. Perbarui konfigurasi atau pilih model lain."
            ),
            ModelError::MissingApiKey { provider } => format!(
                "Penyedia '{provider}' memerlukan API key. Tambahkan field api_key di client.toml sebelum menggunakan penyedia tersebut."
            ),
            ModelError::Network { provider, source } => {
                if source.is_connect() {
                    format!(
                        "Tidak dapat terhubung ke penyedia model '{provider}'. Pastikan layanan berjalan dan endpoint dapat diakses."
                    )
                } else if source.is_timeout() {
                    format!(
                        "Permintaan ke penyedia model '{provider}' melebihi batas waktu. Coba lagi sebentar lagi."
                    )
                } else if let Some(status) = source.status() {
                    match status {
                        StatusCode::NOT_FOUND => format!(
                            "Endpoint penyedia '{provider}' mengembalikan 404. Periksa konfigurasi endpoint dan model."
                        ),
                        StatusCode::SERVICE_UNAVAILABLE | StatusCode::BAD_GATEWAY => format!(
                            "Penyedia model '{provider}' sedang tidak tersedia. Coba lagi nanti."
                        ),
                        _ => format!(
                            "Permintaan ke penyedia model '{provider}' gagal dengan status {}. Coba lagi nanti.",
                            status.as_u16()
                        ),
                    }
                } else {
                    format!(
                        "Terjadi kesalahan jaringan saat menghubungi penyedia '{provider}'. Coba lagi nanti."
                    )
                }
            }
            ModelError::InvalidResponse { provider, .. } => format!(
                "Penyedia model '{provider}' memberikan respons yang tidak dapat diproses. Coba lagi."
            ),
            ModelError::UnsupportedProviderType {
                provider,
                provider_type,
            } => format!(
                "Tipe penyedia '{provider_type}' untuk '{provider}' tidak didukung. Gunakan 'ollama' atau 'gemini'."
            ),
        }
    }
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError>;
}

#[derive(Clone)]
pub struct DynamicModelProvider {
    backends: HashMap<String, ProviderRuntime>,
}

impl DynamicModelProvider {
    pub fn from_configs(configs: &[ModelProviderConfig]) -> Result<Self, ModelError> {
        let mut backends = HashMap::new();
        for config in configs {
            let models = config
                .models
                .iter()
                .map(|model| model.name.clone())
                .collect::<HashSet<_>>();

            let backend = match config.provider_type.to_lowercase().as_str() {
                "ollama" => ProviderBackend::Ollama(OllamaClient::new(
                    config.id.clone(),
                    config.endpoint.clone(),
                )),
                "gemini" => {
                    let resolved = resolve_api_key(config.id.as_str(), config.api_key.as_deref());
                    let api_path = config
                        .api_path
                        .clone()
                        .unwrap_or_else(|| DEFAULT_GEMINI_API_PATH.to_string());
                    ProviderBackend::Gemini(GeminiClient::new(
                        config.id.clone(),
                        config.endpoint.clone(),
                        resolved,
                        api_path,
                    ))
                }
                other => {
                    return Err(ModelError::unsupported_provider_type(
                        config.id.clone(),
                        other.to_string(),
                    ));
                }
            };

            backends.insert(
                config.id.clone(),
                ProviderRuntime {
                    _id: config.id.clone(),
                    _provider_type: config.provider_type.clone(),
                    models,
                    backend,
                },
            );
        }
        Ok(Self { backends })
    }

    pub fn contains(&self, provider: &str) -> bool {
        self.backends.contains_key(provider)
    }
}

#[async_trait]
impl ModelProvider for DynamicModelProvider {
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let provider_id = request.provider.clone();
        let runtime = self
            .backends
            .get(&provider_id)
            .ok_or_else(|| ModelError::provider_not_found(provider_id.clone()))?;
        if !runtime.supports(&request.model) {
            return Err(ModelError::model_not_found(
                provider_id,
                request.model.clone(),
            ));
        }

        runtime.chat(request).await
    }
}

#[derive(Clone)]
struct ProviderRuntime {
    _id: String,
    _provider_type: String,
    models: HashSet<String>,
    backend: ProviderBackend,
}

impl ProviderRuntime {
    fn supports(&self, model: &str) -> bool {
        self.models.is_empty() || self.models.contains(model)
    }

    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        match &self.backend {
            ProviderBackend::Ollama(client) => client.chat(request).await,
            ProviderBackend::Gemini(client) => client.chat(request).await,
        }
    }
}

#[derive(Clone)]
enum ProviderBackend {
    Ollama(OllamaClient),
    Gemini(GeminiClient),
}

#[derive(Clone)]
pub struct OllamaClient {
    id: String,
    http: Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new(id: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self::with_client(id, base_url, Client::new())
    }

    pub fn with_client(id: impl Into<String>, base_url: impl Into<String>, client: Client) -> Self {
        Self {
            id: id.into(),
            http: client,
            base_url: base_url.into(),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        let trimmed = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{trimmed}/{path}")
    }

    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let url = self.endpoint("/api/chat");
        let payload = OllamaChatRequest::from(&request);
        info!(
            provider = self.id.as_str(),
            model = request.model.as_str(),
            url = %url,
            messages = request.messages.len(),
            "Sending request to Ollama provider"
        );
        let response: OllamaChatResponse = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|err| ModelError::network(self.id.clone(), err))?
            .error_for_status()
            .map_err(|err| ModelError::network(self.id.clone(), err))?
            .json()
            .await
            .map_err(|err| ModelError::network(self.id.clone(), err))?;
        debug!("Received response from Ollama provider");

        let message = response.message.ok_or_else(|| {
            ModelError::invalid_response(self.id.clone(), "missing message field")
        })?;

        let role = MessageRole::from_str(message.role.as_str()).ok_or_else(|| {
            ModelError::invalid_response(self.id.clone(), "unknown role in response")
        })?;

        Ok(ModelResponse {
            message: ChatMessage::new(role, message.content),
            session_id: request.session_id,
        })
    }
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
}

impl From<&ModelRequest> for OllamaChatRequest {
    fn from(value: &ModelRequest) -> Self {
        Self {
            model: value.model.clone(),
            messages: value
                .messages
                .iter()
                .map(|msg| OllamaChatMessage {
                    role: msg.role.as_str().to_string(),
                    content: msg.content.clone(),
                })
                .collect(),
            stream: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaChatMessage>,
}

#[derive(Clone)]
pub struct GeminiClient {
    id: String,
    endpoint: String,
    api_key: Option<String>,
    api_path: String,
    http: Client,
}

impl GeminiClient {
    pub fn new(
        id: impl Into<String>,
        endpoint: impl Into<String>,
        api_key: Option<String>,
        api_path: impl Into<String>,
    ) -> Self {
        Self::with_client(id, endpoint, api_key, api_path, Client::new())
    }

    pub fn with_client(
        id: impl Into<String>,
        endpoint: impl Into<String>,
        api_key: Option<String>,
        api_path: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            id: id.into(),
            endpoint: endpoint.into(),
            api_key,
            api_path: api_path.into(),
            http: client,
        }
    }

    fn endpoint(&self, model: &str) -> String {
        let base = self.endpoint.trim_end_matches('/');
        format!("{base}/{}/{model}:generateContent", self.api_path)
    }

    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
        let api_key = match &self.api_key {
            Some(key) if !key.trim().is_empty() => key.clone(),
            _ => return Err(ModelError::missing_api_key(self.id.clone())),
        };

        let url = format!("{}?key={}", self.endpoint(&request.model), api_key);
        let payload = GeminiChatRequest::from(&request);
        info!(
            provider = self.id.as_str(),
            model = request.model.as_str(),
            url = %url,
            messages = request.messages.len(),
            "Sending request to Gemini provider"
        );
        let response: GeminiChatResponse = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|err| ModelError::network(self.id.clone(), err))?
            .error_for_status()
            .map_err(|err| ModelError::network(self.id.clone(), err))?
            .json()
            .await
            .map_err(|err| ModelError::network(self.id.clone(), err))?;
        debug!("Received response from Gemini provider");

        let text = response
            .candidates
            .unwrap_or_default()
            .into_iter()
            .flat_map(|candidate| candidate.content)
            .flat_map(|content| content.parts.into_iter())
            .find_map(|part| part.text)
            .ok_or_else(|| {
                ModelError::invalid_response(self.id.clone(), "missing text in response")
            })?;

        Ok(ModelResponse {
            message: ChatMessage::new(MessageRole::Assistant, text),
            session_id: request.session_id,
        })
    }
}

#[derive(Debug, Serialize)]
struct GeminiChatRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiInstruction>,
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

impl From<&ModelRequest> for GeminiChatRequest {
    fn from(request: &ModelRequest) -> Self {
        let mut system_parts = Vec::new();
        let mut contents = Vec::new();

        for message in &request.messages {
            match message.role {
                MessageRole::System => system_parts.push(message.content.clone()),
                MessageRole::User => contents.push(GeminiContent {
                    role: "user".to_string(),
                    parts: vec![GeminiPart {
                        text: message.content.clone(),
                    }],
                }),
                MessageRole::Assistant => contents.push(GeminiContent {
                    role: "model".to_string(),
                    parts: vec![GeminiPart {
                        text: message.content.clone(),
                    }],
                }),
            }
        }

        let system_instruction = if system_parts.is_empty() {
            None
        } else {
            Some(GeminiInstruction {
                parts: vec![GeminiPart {
                    text: system_parts.join("\n\n"),
                }],
            })
        };

        Self {
            system_instruction,
            contents,
            generation_config: GeminiGenerationConfig::json_response(),
        }
    }
}

fn resolve_api_key(provider: &str, spec: Option<&str>) -> Option<String> {
    let Some(raw) = spec.map(str::trim) else {
        return None;
    };
    if raw.is_empty() {
        return None;
    }
    match env::var(raw) {
        Ok(value) => Some(value),
        Err(err) => {
            warn!(
                provider,
                env_var = raw,
                %err,
                "API key environment variable is not set or unreadable"
            );
            None
        }
    }
}

#[derive(Debug, Serialize)]
struct GeminiInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
}

impl GeminiGenerationConfig {
    fn json_response() -> Self {
        Self {
            response_mime_type: "application/json".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GeminiChatResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiCandidateContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidateContent {
    #[serde(default)]
    parts: Vec<GeminiCandidatePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidatePart {
    #[serde(default)]
    text: Option<String>,
}

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Trait for configuration types that can be parsed from TOML.
/// This provides a consistent interface for converting raw deserialized
/// data into the final configuration types.
pub trait ParseableConfig<R>: Sized
where
    R: for<'de> Deserialize<'de>,
{
    /// Convert from the raw deserialized type to the final config type
    fn from_raw(raw: R) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ModelInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ModelProviderConfig {
    pub id: String,
    /// The provider type (e.g., "ollama", "gemini", "openai", etc.)
    #[serde(rename = "type")]
    pub provider_type: String,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Custom API path (e.g., "v1beta/models" for Gemini)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_path: Option<String>,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RawProviderConfig {
    pub(super) id: String,
    #[serde(rename = "type", default)]
    pub(super) provider_type: String,
    pub(super) endpoint: Option<String>,
    pub(super) api_key: Option<String>,
    #[serde(default)]
    pub(super) api_path: Option<String>,
    #[serde(default)]
    pub(super) models: Vec<RawModelInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(super) enum RawModelInfo {
    Name(String),
    Detailed {
        name: String,
        #[serde(default)]
        display_name: Option<String>,
    },
}

impl From<RawModelInfo> for ModelInfo {
    fn from(value: RawModelInfo) -> Self {
        match value {
            RawModelInfo::Name(name) => Self {
                name,
                display_name: None,
            },
            RawModelInfo::Detailed { name, display_name } => Self { name, display_name },
        }
    }
}

impl From<RawProviderConfig> for ModelProviderConfig {
    fn from(raw: RawProviderConfig) -> Self {
        let endpoint = raw.endpoint.unwrap_or_default();

        Self {
            id: raw.id,
            provider_type: raw.provider_type,
            endpoint,
            api_key: raw.api_key,
            api_path: raw.api_path,
            models: raw.models.into_iter().map(ModelInfo::from).collect(),
        }
    }
}

impl ModelProviderConfig {
    /// Ensure a model exists in this provider's model list
    pub fn ensure_model(&mut self, model: &str) {
        if self.models.iter().all(|info| info.name != model) {
            self.models.push(ModelInfo {
                name: model.to_string(),
                display_name: None,
            });
        }
    }

    /// Check if this is an Ollama provider (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// use antikhitera_mcp_client::config::ModelProviderConfig;
    ///
    /// let provider = ModelProviderConfig {
    ///     id: "local".to_string(),
    ///     provider_type: "ollama".to_string(),
    ///     endpoint: "http://localhost:11434".to_string(),
    ///     api_key: None,
    ///     api_path: None,
    ///     models: vec![],
    /// };
    /// assert!(provider.is_ollama());
    /// assert!(!provider.is_gemini());
    /// ```
    pub fn is_ollama(&self) -> bool {
        self.provider_type.eq_ignore_ascii_case("ollama")
    }

    /// Check if this is a Gemini provider (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// use antikhitera_mcp_client::config::ModelProviderConfig;
    ///
    /// let provider = ModelProviderConfig {
    ///     id: "google".to_string(),
    ///     provider_type: "GEMINI".to_string(),
    ///     endpoint: "https://example.com".to_string(),
    ///     api_key: Some("key".to_string()),
    ///     api_path: None,
    ///     models: vec![],
    /// };
    /// assert!(provider.is_gemini());
    /// assert!(!provider.is_ollama());
    /// ```
    pub fn is_gemini(&self) -> bool {
        self.provider_type.eq_ignore_ascii_case("gemini")
    }
}

//! # Provider Configuration
//!
//! This module defines the configuration types for AI model providers.
//! Supported provider types include Gemini, OpenAI, and Ollama.
//!
//! ## Provider Types
//!
//! | Type | Description | API Key Required |
//! |------|-------------|-----------------|
//! | `gemini` | Google Gemini API | Yes |
//! | `openai` | OpenAI-compatible APIs | Yes |
//! | `ollama` | Local Ollama server | No |

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Trait for configuration types that can be parsed from TOML.
///
/// This provides a consistent interface for converting raw deserialized
/// data into the final configuration types.
pub trait ParseableConfig<R>: Sized
where
    R: for<'de> Deserialize<'de>,
{
    /// Convert from the raw deserialized type to the final config type
    fn from_raw(raw: R) -> Self;
}

/// Information about an available model from a provider.
///
/// Models can be specified with just a name, or with an optional display name
/// for better UI presentation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ModelInfo {
    /// Model identifier used in API calls (e.g., "gemini-2.0-flash")
    pub name: String,
    /// Human-readable display name for UI (e.g., "Gemini 2.0 Flash")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Configuration for an AI model provider.
///
/// Each provider represents a connection to an AI service endpoint.
/// Multiple providers can be configured to allow switching between
/// different models or services.
///
/// # Example
///
/// ```toml
/// [[providers]]
/// id = "gemini"
/// type = "gemini"
/// endpoint = "https://generativelanguage.googleapis.com"
/// api_key = "${GEMINI_API_KEY}"
/// models = [
///     { name = "gemini-2.0-flash", display_name = "Gemini 2.0 Flash" }
/// ]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ModelProviderConfig {
    /// Unique identifier for this provider (e.g., "gemini", "ollama-local")
    pub id: String,
    /// The provider type determines API format: "ollama", "gemini", "openai"
    #[serde(rename = "type")]
    pub provider_type: String,
    /// API endpoint URL
    pub endpoint: String,
    /// API key (can use environment variable syntax like "${VAR_NAME}")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Custom API path override (e.g., "v1beta/models" for Gemini)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_path: Option<String>,
    /// List of available models from this provider
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

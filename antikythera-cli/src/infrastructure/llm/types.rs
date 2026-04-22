//! CLI-owned runtime provider configuration types.
//!
//! `ModelProviderConfig` and `ModelInfo` represent the runtime connection
//! parameters for an LLM backend. These types live in the CLI layer because
//! provider-specific wiring (endpoints, API keys, model lists) is a CLI
//! concern — `antikythera-core` is completely agnostic about which LLM is
//! speaking to it.

use serde::{Deserialize, Serialize};

use crate::config::{ModelInfo as PostcardModelInfo, ProviderConfig};

/// Runtime connection parameters for a single LLM provider backend.
///
/// Converted from the serialised [`ProviderConfig`] that lives in `app.pc`.
/// The CLI LLM clients (Gemini, Ollama, OpenAI) all accept this type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelProviderConfig {
    /// Unique provider ID used as a routing key (e.g. `"gemini"`, `"ollama"`).
    pub id: String,
    /// Provider type tag used by [`ProviderFactory`] to choose the client
    /// implementation (e.g. `"gemini"`, `"ollama"`, `"openai"`).
    pub provider_type: String,
    /// Base API endpoint URL.
    pub endpoint: String,
    /// Resolved API key value (not an env-var name — already looked up).
    pub api_key: Option<String>,
    /// Optional provider-specific API path override (used by Gemini).
    pub api_path: Option<String>,
    /// Models offered by this provider.
    pub models: Vec<ModelInfo>,
}

impl ModelProviderConfig {
    /// Returns `true` when the provider type indicates an Ollama-compatible
    /// backend.
    pub fn is_ollama(&self) -> bool {
        matches!(
            self.provider_type.to_lowercase().as_str(),
            "ollama" | "localai"
        )
    }

    /// Returns `true` when the provider type indicates a Gemini / Google AI
    /// backend.
    pub fn is_gemini(&self) -> bool {
        matches!(
            self.provider_type.to_lowercase().as_str(),
            "gemini" | "google" | "google-ai"
        )
    }

    /// Ensure the given model name appears in the models list.
    /// Appends it if it is absent, so the runtime never rejects the selection.
    pub fn ensure_model(&mut self, model: &str) {
        if !self.models.iter().any(|m| m.name == model) {
            self.models.push(ModelInfo {
                name: model.to_string(),
                display_name: None,
            });
        }
    }
}

/// A single model entry within a [`ModelProviderConfig`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    pub name: String,
    pub display_name: Option<String>,
}

// ── Conversions from postcard serialisation types ────────────────────────────

impl From<&ProviderConfig> for ModelProviderConfig {
    fn from(pc: &ProviderConfig) -> Self {
        Self {
            id: pc.id.clone(),
            provider_type: pc.provider_type.clone(),
            endpoint: pc.endpoint.clone(),
            api_key: if pc.api_key.is_empty() {
                None
            } else {
                Some(pc.api_key.clone())
            },
            api_path: None,
            models: pc.models.iter().map(ModelInfo::from).collect(),
        }
    }
}

impl From<ModelProviderConfig> for ProviderConfig {
    fn from(mp: ModelProviderConfig) -> Self {
        Self {
            id: mp.id,
            provider_type: mp.provider_type,
            endpoint: mp.endpoint,
            api_key: mp.api_key.unwrap_or_default(),
            models: mp.models.into_iter().map(PostcardModelInfo::from).collect(),
        }
    }
}

impl From<&PostcardModelInfo> for ModelInfo {
    fn from(pm: &PostcardModelInfo) -> Self {
        Self {
            name: pm.name.clone(),
            display_name: if pm.display_name.is_empty() {
                None
            } else {
                Some(pm.display_name.clone())
            },
        }
    }
}

impl From<ModelInfo> for PostcardModelInfo {
    fn from(mi: ModelInfo) -> Self {
        Self {
            name: mi.name,
            display_name: mi.display_name.unwrap_or_default(),
        }
    }
}

/// Convert a slice of postcard [`ProviderConfig`]s to runtime
/// [`ModelProviderConfig`]s.
pub fn providers_from_postcard(configs: &[ProviderConfig]) -> Vec<ModelProviderConfig> {
    configs.iter().map(ModelProviderConfig::from).collect()
}

/// Convert runtime [`ModelProviderConfig`]s back to postcard [`ProviderConfig`]s
/// for persistence.
pub fn providers_to_postcard(configs: Vec<ModelProviderConfig>) -> Vec<ProviderConfig> {
    configs.into_iter().map(ProviderConfig::from).collect()
}

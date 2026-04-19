//! Base HTTP client — CLI's own copy for LLM provider calls
//!
//! This is the **CLI-side** HTTP transport layer for LLM providers.  It lives
//! here rather than in `antikythera-core` so that the core crate can be
//! compiled to WASM without any HTTP client code.
//!
//! The `reqwest` error type is converted to a plain `String` before being
//! wrapped in `ModelError::network`, keeping `ModelError` free of
//! `reqwest`-specific types.

use antikythera_core::infrastructure::model::types::ModelError;
use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Base HTTP client with shared functionality for LLM provider calls.
#[derive(Clone)]
pub struct HttpClientBase {
    pub id: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub http: Client,
}

impl HttpClientBase {
    pub fn new(id: String, endpoint: String, api_key: Option<String>) -> Self {
        Self {
            id,
            endpoint,
            api_key,
            http: Client::new(),
        }
    }

    /// Build a URL from the base endpoint and a relative path.
    pub fn build_url(&self, path: &str) -> String {
        let base = self.endpoint.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{base}/{path}")
    }

    /// POST JSON with bearer auth and return raw response body as text.
    pub async fn post_with_bearer_text<Req>(
        &self,
        url: &str,
        body: &Req,
    ) -> Result<String, ModelError>
    where
        Req: Serialize,
    {
        let api_key = self.require_api_key()?;

        self.http
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| ModelError::network(&self.id, e.to_string()))?
            .error_for_status()
            .map_err(|e| ModelError::network(&self.id, e.to_string()))?
            .text()
            .await
            .map_err(|e| ModelError::network(&self.id, e.to_string()))
    }

    /// POST JSON with `?key=<api_key>` query parameter auth (Gemini style).
    pub async fn post_with_query_key<Req, Res>(
        &self,
        url: &str,
        body: &Req,
    ) -> Result<Res, ModelError>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        let api_key = self.require_api_key()?;
        let url_with_key = format!("{}?key={}", url, api_key);

        self.http
            .post(&url_with_key)
            .json(body)
            .send()
            .await
            .map_err(|e| ModelError::network(&self.id, e.to_string()))?
            .error_for_status()
            .map_err(|e| ModelError::network(&self.id, e.to_string()))?
            .json()
            .await
            .map_err(|e| ModelError::network(&self.id, e.to_string()))
    }

    /// POST JSON without auth and return raw response body as text.
    pub async fn post_no_auth_text<Req>(&self, url: &str, body: &Req) -> Result<String, ModelError>
    where
        Req: Serialize,
    {
        self.http
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| ModelError::network(&self.id, e.to_string()))?
            .error_for_status()
            .map_err(|e| ModelError::network(&self.id, e.to_string()))?
            .text()
            .await
            .map_err(|e| ModelError::network(&self.id, e.to_string()))
    }

    fn require_api_key(&self) -> Result<&str, ModelError> {
        self.api_key
            .as_deref()
            .filter(|k| !k.trim().is_empty())
            .ok_or_else(|| ModelError::missing_api_key(&self.id))
    }
}

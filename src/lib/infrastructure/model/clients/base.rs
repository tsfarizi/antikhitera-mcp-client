//! Base HTTP client with shared logic

use crate::infrastructure::model::types::ModelError;
use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Base HTTP client with shared functionality
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

    /// Build URL from endpoint and path
    pub fn build_url(&self, path: &str) -> String {
        let base = self.endpoint.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{base}/{path}")
    }

    /// Post JSON with bearer auth
    pub async fn post_with_bearer<Req, Res>(&self, url: &str, body: &Req) -> Result<Res, ModelError>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        let api_key = self.require_api_key()?;

        self.http
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| ModelError::network(&self.id, e))?
            .error_for_status()
            .map_err(|e| ModelError::network(&self.id, e))?
            .json()
            .await
            .map_err(|e| ModelError::network(&self.id, e))
    }

    /// Post JSON with query param auth (for Gemini)
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
            .map_err(|e| ModelError::network(&self.id, e))?
            .error_for_status()
            .map_err(|e| ModelError::network(&self.id, e))?
            .json()
            .await
            .map_err(|e| ModelError::network(&self.id, e))
    }

    /// Post JSON without auth (for local services like Ollama)
    pub async fn post_no_auth<Req, Res>(&self, url: &str, body: &Req) -> Result<Res, ModelError>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        self.http
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| ModelError::network(&self.id, e))?
            .error_for_status()
            .map_err(|e| ModelError::network(&self.id, e))?
            .json()
            .await
            .map_err(|e| ModelError::network(&self.id, e))
    }

    fn require_api_key(&self) -> Result<&str, ModelError> {
        self.api_key
            .as_deref()
            .filter(|k| !k.trim().is_empty())
            .ok_or_else(|| ModelError::missing_api_key(&self.id))
    }
}

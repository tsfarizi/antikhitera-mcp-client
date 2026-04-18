//! MCP Client Feature Slice
//!
//! Provides **browser WASM** bindings for MCP client operations via `wasm-bindgen`.
//!
//! ## Target clarification
//!
//! This module is compiled ONLY when the `wasm` feature is active
//! (target `wasm32-unknown-unknown` / browser environment).
//!
//! It is NOT part of the primary WASM path. The primary WASM target for this
//! framework is the server-side WASM component (`component` feature,
//! `wasm32-wasip1`), which communicates with its host through WIT-defined
//! imports (`antikythera::call_llm_sync`, etc.) declared in `wit/antikythera.wit`.
//!
//! This browser-facing client exists for optional web embedding only.

use wasm_bindgen::prelude::*;
use antikythera_core::application::agent::{Agent, AgentOptions};
use antikythera_core::application::client::{ClientConfig, McpClient};
use antikythera_core::infrastructure::model::DynamicModelProvider;
use serde_json::json;
use std::sync::Arc;
use wasm_bindgen_futures::future_to_promise;

/// WASM-compatible MCP client
#[wasm_bindgen]
pub struct WasmClient {
    core_client: Arc<McpClient<DynamicModelProvider>>,
}

#[wasm_bindgen]
impl WasmClient {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str) -> Result<WasmClient, JsValue> {
        console_error_panic_hook::set_once();

        let config: serde_json::Value = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON config: {}", e)))?;

        let providers_value = config.get("providers")
            .ok_or_else(|| JsValue::from_str("Missing 'providers' in config"))?;

        let providers: Vec<antikythera_core::config::ModelProviderConfig> =
            serde_json::from_value(providers_value.clone())
                .map_err(|e| JsValue::from_str(&format!("Invalid providers: {}", e)))?;

        let provider = DynamicModelProvider::from_configs(&providers)
            .map_err(|e| JsValue::from_str(&format!("Failed to create provider: {}", e)))?;

        let default_provider = config.get("default_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("ollama")
            .to_string();

        let model = config.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("llama3")
            .to_string();

        let client_config = ClientConfig::new(default_provider, model);
        let core_client = Arc::new(McpClient::new(provider, client_config));

        Ok(Self { core_client })
    }

    #[wasm_bindgen]
    pub fn chat(&self, prompt: String) -> js_sys::Promise {
        let client = self.core_client.clone();

        future_to_promise(async move {
            let request = antikythera_core::application::client::ChatRequest {
                prompt,
                attachments: Vec::new(),
                system_prompt: None,
                session_id: None,
                raw_mode: false,
                bypass_template: false,
                force_json: false,
                correlation_id: None,
                tools: Vec::new(),
                tool_choice: None,
            };

            let response = client.chat(request)
                .await
                .map_err(|e| JsValue::from_str(&format!("Chat error: {}", e)))?;

            Ok(JsValue::from_str(&response.content))
        })
    }

    #[wasm_bindgen]
    pub fn run_agent(&self, prompt: String, options_json: &str) -> js_sys::Promise {
        let client = self.core_client.clone();
        let options: AgentOptions = serde_json::from_str(options_json).unwrap_or_default();

        future_to_promise(async move {
            let agent = Agent::new(client);
            let outcome = agent.run(prompt, options)
                .await
                .map_err(|e| JsValue::from_str(&format!("Agent error: {}", e)))?;

            let result_json = json!({
                "response": outcome.response,
                "logs": outcome.logs,
                "session_id": outcome.session_id,
            });

            Ok(JsValue::from_str(&result_json.to_string()))
        })
    }

    #[wasm_bindgen]
    pub fn list_tools(&self) -> js_sys::Promise {
        let tools = self.core_client.tools().to_vec();

        future_to_promise(async move {
            let tools_json = json!({
                "tools": tools.iter().map(|t| json!({
                    "name": t.name,
                    "description": t.description,
                })).collect::<Vec<_>>()
            });

            Ok(JsValue::from_str(&tools_json.to_string()))
        })
    }

    #[wasm_bindgen(js_name = getPromptTemplate)]
    pub fn get_prompt_template(&self) -> Result<String, JsValue> {
        use antikythera_core::config::app::AppConfig;
        use std::path::Path;

        let config_path = Path::new(antikythera_core::constants::CONFIG_PATH);
        let config = AppConfig::load(Some(config_path))
            .map_err(|e| JsValue::from_str(&format!("Failed to load config: {}", e)))?;

        Ok(config.prompt_template().to_string())
    }
}

/// Initialize WASM client
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

use super::CONFIG_PATH;
use super::provider::*;
use super::server::*;
use super::tool::*;
use dotenvy::from_filename;
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Once;
use thiserror::Error;
use tracing::debug;

static ENV_LOADER: Once = Once::new();

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub default_provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub tools: Vec<ToolConfig>,
    pub servers: Vec<ServerConfig>,
    pub prompt_template: String,
    pub providers: Vec<ModelProviderConfig>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("configuration file not found at {path:?}")]
    NotFound { path: PathBuf },
    #[error("failed to read config from {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse config from {path:?}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("missing required field 'model' in configuration")]
    MissingModel,
    #[error("missing required field 'default_provider' in configuration")]
    MissingDefaultProvider,
    #[error("missing required field 'prompt_template' in configuration")]
    MissingPromptTemplate,
    #[error("no providers configured - at least one [[providers]] entry is required")]
    NoProvidersConfigured,
    #[error("default provider '{provider}' not found in configured providers")]
    ProviderNotFound { provider: String },
    #[error("provider '{provider}' is missing required field 'endpoint'")]
    MissingEndpoint { provider: String },
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct RawConfig {
    model: Option<String>,
    default_provider: Option<String>,
    system_prompt: Option<String>,
    #[serde(default)]
    tools: Vec<RawTool>,
    #[serde(default)]
    servers: Vec<RawServer>,
    prompt_template: Option<String>,
    #[serde(default)]
    providers: Vec<RawProviderConfig>,
}

impl AppConfig {
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        ensure_env_loaded();
        let config_path = path.unwrap_or_else(|| Path::new(CONFIG_PATH));
        read_config(config_path)
    }

    pub fn prompt_template(&self) -> &str {
        &self.prompt_template
    }

    pub fn to_raw_toml(&self) -> String {
        to_raw_toml_string(self)
    }
}

pub fn to_raw_toml_string(config: &AppConfig) -> String {
    render_config_raw(
        &config.default_provider,
        &config.model,
        config.system_prompt.as_deref(),
        &config.prompt_template,
        &config.tools,
        &config.providers,
    )
}

fn ensure_env_loaded() {
    ENV_LOADER.call_once(|| {
        let _ = from_filename("config/.env");
    });
}

fn read_config(path: &Path) -> Result<AppConfig, ConfigError> {
    debug!(path = %path.display(), "Reading client configuration file");

    let content = fs::read_to_string(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            ConfigError::NotFound {
                path: path.to_path_buf(),
            }
        } else {
            ConfigError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
    })?;

    let parsed: RawConfig = toml::from_str(&content).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    // Required fields validation
    let model = parsed.model.ok_or(ConfigError::MissingModel)?;
    let default_provider = parsed
        .default_provider
        .ok_or(ConfigError::MissingDefaultProvider)?;
    let prompt_template = parsed
        .prompt_template
        .ok_or(ConfigError::MissingPromptTemplate)?;

    if parsed.providers.is_empty() {
        return Err(ConfigError::NoProvidersConfigured);
    }

    let mut providers: Vec<ModelProviderConfig> = Vec::new();
    for raw_provider in parsed.providers {
        if raw_provider.endpoint.is_none() {
            return Err(ConfigError::MissingEndpoint {
                provider: raw_provider.id.clone(),
            });
        }
        providers.push(ModelProviderConfig::from(raw_provider));
    }

    // Validate default provider exists
    if !providers.iter().any(|p| p.id == default_provider) {
        return Err(ConfigError::ProviderNotFound {
            provider: default_provider,
        });
    }

    // Ensure model exists in default provider
    if let Some(provider) = providers.iter_mut().find(|p| p.id == default_provider) {
        provider.ensure_model(&model);
    }

    Ok(AppConfig {
        default_provider,
        model,
        system_prompt: parsed.system_prompt,
        tools: parsed.tools.into_iter().map(ToolConfig::from).collect(),
        servers: parsed.servers.into_iter().map(ServerConfig::from).collect(),
        prompt_template,
        providers,
    })
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
            raw.push_str(&format!("type = \"{}\"\n", escape(&provider.provider_type)));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    fn write_valid_config(path: &Path) {
        let mut file = File::create(path).expect("create config");
        writeln!(
            file,
            r#"
default_provider = "gemini"
model = "gemini-1.5-flash"
prompt_template = "You are a helpful assistant."

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://generativelanguage.googleapis.com"
api_key = "TEST_KEY"
models = [{{ name = "gemini-1.5-flash" }}]
"#
        )
        .expect("write");
    }

    #[test]
    fn errors_when_config_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nonexistent.toml");

        let result = AppConfig::load(Some(&path));
        assert!(matches!(result, Err(ConfigError::NotFound { .. })));
    }

    #[test]
    fn errors_when_model_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
default_provider = "gemini"
prompt_template = "test"

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["test"]
"#,
        )
        .expect("write");

        let result = AppConfig::load(Some(&path));
        assert!(matches!(result, Err(ConfigError::MissingModel)));
    }

    #[test]
    fn errors_when_default_provider_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
model = "test-model"
prompt_template = "test"

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["test"]
"#,
        )
        .expect("write");

        let result = AppConfig::load(Some(&path));
        assert!(matches!(result, Err(ConfigError::MissingDefaultProvider)));
    }

    #[test]
    fn errors_when_prompt_template_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
model = "test-model"
default_provider = "gemini"

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["test"]
"#,
        )
        .expect("write");

        let result = AppConfig::load(Some(&path));
        assert!(matches!(result, Err(ConfigError::MissingPromptTemplate)));
    }

    #[test]
    fn errors_when_no_providers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
model = "test-model"
default_provider = "gemini"
prompt_template = "test"
"#,
        )
        .expect("write");

        let result = AppConfig::load(Some(&path));
        assert!(matches!(result, Err(ConfigError::NoProvidersConfigured)));
    }

    #[test]
    fn errors_when_provider_missing_endpoint() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
model = "test-model"
default_provider = "gemini"
prompt_template = "test"

[[providers]]
id = "gemini"
type = "gemini"
models = ["test"]
"#,
        )
        .expect("write");

        let result = AppConfig::load(Some(&path));
        assert!(matches!(result, Err(ConfigError::MissingEndpoint { .. })));
    }

    #[test]
    fn errors_when_default_provider_not_in_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
model = "test-model"
default_provider = "nonexistent"
prompt_template = "test"

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://example.com"
models = ["test"]
"#,
        )
        .expect("write");

        let result = AppConfig::load(Some(&path));
        assert!(matches!(result, Err(ConfigError::ProviderNotFound { .. })));
    }

    #[test]
    fn loads_valid_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        write_valid_config(&path);

        let config = AppConfig::load(Some(&path)).expect("load config");
        assert_eq!(config.model, "gemini-1.5-flash");
        assert_eq!(config.default_provider, "gemini");
        assert_eq!(config.prompt_template, "You are a helpful assistant.");
        assert_eq!(config.providers.len(), 1);
        assert!(config.providers[0].is_gemini());
    }

    #[test]
    fn reads_server_definitions_and_tool_bindings() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
model = "mistral"
default_provider = "ollama"
prompt_template = "Be helpful."

[[providers]]
id = "ollama"
type = "ollama"
endpoint = "http://localhost:11434"
models = ["mistral"]

[[servers]]
name = "time"
command = "server.exe"
args = ["--flag"]
workdir = "C:/work"
default_timezone = "Asia/Jakarta"
default_city = "Jakarta"

[[servers]]
name = "other"
command = "other.exe"

[[tools]]
name = "get_time"
description = "Fetch time"
server = "time"
"#,
        )
        .expect("write servers config");

        let config = AppConfig::load(Some(&path)).expect("load config");
        assert_eq!(config.servers.len(), 2);
        assert_eq!(config.servers[0].name, "time");
        assert_eq!(config.servers[0].command, PathBuf::from("server.exe"));
        assert_eq!(config.servers[0].args, vec!["--flag"]);
        assert_eq!(
            config.servers[0].workdir.as_deref(),
            Some(Path::new("C:/work"))
        );
        assert_eq!(
            config.servers[0].default_timezone.as_deref(),
            Some("Asia/Jakarta")
        );
        assert_eq!(config.servers[0].default_city.as_deref(), Some("Jakarta"));
        assert_eq!(config.servers[1].name, "other");

        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.tools[0].name, "get_time");
        assert_eq!(config.tools[0].server.as_deref(), Some("time"));
    }

    #[test]
    fn reads_provider_definitions() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
model = "gemini-1.5-flash"
default_provider = "gemini"
prompt_template = "Be helpful."

[[providers]]
id = "ollama"
type = "ollama"
endpoint = "http://localhost:11434"
models = ["llama3"]

[[providers]]
id = "gemini"
type = "gemini"
endpoint = "https://generativelanguage.googleapis.com"
api_key = "secret"
models = [{ name = "gemini-1.5-flash", display_name = "Gemini Flash" }]
"#,
        )
        .expect("write provider config");

        let config = AppConfig::load(Some(&path)).expect("load provider config");
        assert_eq!(config.model, "gemini-1.5-flash");
        assert_eq!(config.default_provider, "gemini");
        assert_eq!(config.providers.len(), 2);
        let gemini = config
            .providers
            .iter()
            .find(|provider| provider.id == "gemini")
            .expect("gemini provider exists");
        assert!(gemini.is_gemini());
        assert_eq!(gemini.provider_type, "gemini");
        assert_eq!(gemini.api_key.as_deref(), Some("secret"));
        assert_eq!(
            gemini
                .models
                .iter()
                .find(|model| model.name == "gemini-1.5-flash")
                .and_then(|model| model.display_name.as_deref()),
            Some("Gemini Flash")
        );
    }
}

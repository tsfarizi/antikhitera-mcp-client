use super::defaults::*;
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
use tracing::{debug, info};

static ENV_LOADER: Once = Once::new();

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub default_provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub tools: Vec<ToolConfig>,
    pub servers: Vec<ServerConfig>,
    pub prompt_template: Option<String>,
    pub providers: Vec<ModelProviderConfig>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
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
        if let Some(path) = path {
            return read_config(path);
        }
        let default_path = Path::new(DEFAULT_CONFIG_PATH);
        match read_config(default_path) {
            Ok(config) => Ok(config),
            Err(ConfigError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                info!("Configuration file not found; using defaults");
                Ok(Self::default())
            }
            Err(other) => Err(other),
        }
    }

    pub fn default() -> Self {
        Self {
            default_provider: DEFAULT_PROVIDER_ID.to_string(),
            model: DEFAULT_MODEL.to_string(),
            system_prompt: None,
            tools: Vec::new(),
            servers: Vec::new(),
            prompt_template: Some(DEFAULT_PROMPT_TEMPLATE.to_string()),
            providers: vec![default_gemini_provider(), default_ollama_provider()],
        }
    }

    pub fn prompt_template_or_default(&self) -> &str {
        self.prompt_template
            .as_deref()
            .unwrap_or(DEFAULT_PROMPT_TEMPLATE)
    }

    pub fn to_raw_toml(&self) -> String {
        to_raw_toml_string(self)
    }
}

/// Create a default Gemini provider configuration
pub fn default_gemini_provider() -> ModelProviderConfig {
    ModelProviderConfig {
        id: "gemini".to_string(),
        provider_type: "gemini".to_string(),
        endpoint: DEFAULT_GEMINI_ENDPOINT.to_string(),
        api_key: Some("GEMINI_API_KEY".to_string()),
        models: vec![ModelInfo {
            name: DEFAULT_MODEL.to_string(),
            display_name: Some("Gemini 1.5 Flash".to_string()),
        }],
    }
}

/// Create a default Ollama provider configuration
pub fn default_ollama_provider() -> ModelProviderConfig {
    ModelProviderConfig {
        id: "ollama".to_string(),
        provider_type: "ollama".to_string(),
        endpoint: DEFAULT_OLLAMA_ENDPOINT.to_string(),
        api_key: None,
        models: vec![ModelInfo {
            name: "llama3".to_string(),
            display_name: Some("Llama 3".to_string()),
        }],
    }
}

pub fn to_raw_toml_string(config: &AppConfig) -> String {
    render_config_raw(
        &config.default_provider,
        &config.model,
        config.system_prompt.as_deref(),
        config.prompt_template_or_default(),
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
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let parsed: RawConfig = toml::from_str(&content).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    let mut providers: Vec<ModelProviderConfig> = if parsed.providers.is_empty() {
        vec![default_gemini_provider(), default_ollama_provider()]
    } else {
        parsed
            .providers
            .into_iter()
            .map(ModelProviderConfig::from)
            .collect()
    };
    let model = parsed.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let mut default_provider = parsed
        .default_provider
        .or_else(|| {
            providers
                .iter()
                .find(|provider| provider.models.iter().any(|m| m.name == model))
                .map(|provider| provider.id.clone())
        })
        .unwrap_or_else(|| DEFAULT_PROVIDER_ID.to_string());

    if let Some(provider) = providers.iter_mut().find(|p| p.id == default_provider) {
        provider.ensure_model(&model);
    } else {
        let mut fallback = default_ollama_provider();
        fallback.ensure_model(&model);
        default_provider = fallback.id.clone();
        providers.push(fallback);
    }

    Ok(AppConfig {
        default_provider,
        model,
        system_prompt: parsed.system_prompt,
        tools: parsed.tools.into_iter().map(ToolConfig::from).collect(),
        servers: parsed.servers.into_iter().map(ServerConfig::from).collect(),
        prompt_template: Some(
            parsed
                .prompt_template
                .unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string()),
        ),
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
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use std::sync::Mutex;

    static WORKDIR_GUARD: Mutex<()> = Mutex::new(());

    #[test]
    fn returns_default_when_missing() {
        let _lock = WORKDIR_GUARD.lock().expect("lock guard");
        let original_dir = env::current_dir().expect("current dir");
        let temp = tempfile::tempdir().expect("tempdir");
        env::set_current_dir(temp.path()).expect("switch to temp dir");

        let config = AppConfig::load(None).expect("load succeeds");
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.default_provider, DEFAULT_PROVIDER_ID);
        assert!(!config.providers.is_empty());
        assert!(
            config
                .providers
                .iter()
                .any(|provider| provider.id == DEFAULT_PROVIDER_ID)
        );
        assert!(config.providers.iter().any(|provider| {
            provider
                .models
                .iter()
                .any(|model| model.name == DEFAULT_MODEL)
        }));
        assert!(config.system_prompt.is_none());
        assert!(config.tools.is_empty());
        assert!(config.servers.is_empty());
        assert_eq!(
            config.prompt_template.as_deref(),
            Some(DEFAULT_PROMPT_TEMPLATE)
        );

        env::set_current_dir(original_dir).expect("restore current dir");
    }

    #[test]
    fn reads_model_and_system_prompt() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        let mut file = File::create(&path).expect("create config");
        writeln!(
            file,
            r#"
model = "mistral"
system_prompt = "keep short"
"#
        )
        .expect("write");

        let config = AppConfig::load(Some(&path)).expect("load config");
        assert_eq!(config.model, "mistral");
        assert!(
            config
                .providers
                .iter()
                .any(|provider| provider.id == config.default_provider)
        );
        assert_eq!(config.system_prompt.as_deref(), Some("keep short"));
        assert!(config.tools.is_empty());
        assert!(config.servers.is_empty());
        assert_eq!(
            config.prompt_template.as_deref(),
            Some(DEFAULT_PROMPT_TEMPLATE)
        );
    }

    #[test]
    fn falls_back_to_default_model_if_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(&path, "system_prompt = \"only system\"").expect("write");

        let config = AppConfig::load(Some(&path)).expect("load");
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.default_provider, DEFAULT_PROVIDER_ID);
        assert!(
            config
                .providers
                .iter()
                .any(|provider| provider.id == config.default_provider)
        );
        assert_eq!(config.system_prompt.as_deref(), Some("only system"));
        assert!(config.tools.is_empty());
        assert!(config.servers.is_empty());
        assert_eq!(
            config.prompt_template.as_deref(),
            Some(DEFAULT_PROMPT_TEMPLATE)
        );
    }

    #[test]
    fn reads_tool_definitions() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
model = "mistral"

tools = [
    "tool-a",
    { name = "tool-b", description = "Second tool" }
]
"#,
        )
        .expect("write tools config");

        let config = AppConfig::load(Some(&path)).expect("load");
        assert_eq!(config.tools.len(), 2);
        assert_eq!(config.tools[0].name, "tool-a");
        assert!(config.tools[0].description.is_none());
        assert!(config.tools[0].server.is_none());
        assert_eq!(config.tools[1].name, "tool-b");
        assert_eq!(config.tools[1].description.as_deref(), Some("Second tool"));
        assert!(config.tools[1].server.is_none());
        assert_eq!(
            config.prompt_template.as_deref(),
            Some(DEFAULT_PROMPT_TEMPLATE)
        );
        assert!(config.servers.is_empty());
    }

    #[test]
    fn reads_server_definitions_and_tool_bindings() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("client.toml");
        fs::write(
            &path,
            r#"
model = "mistral"

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
        assert!(config.servers[1].workdir.is_none());
        assert!(config.servers[1].default_timezone.is_none());
        assert!(config.servers[1].default_city.is_none());

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

[[providers]]
id = "ollama"
type = "ollama"
endpoint = "http://localhost:11434"
models = ["llama3"]

[[providers]]
id = "gemini"
type = "gemini"
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
        assert!(config.system_prompt.is_none());
        assert!(config.tools.is_empty());
        assert!(config.servers.is_empty());
        assert_eq!(
            config.prompt_template.as_deref(),
            Some(DEFAULT_PROMPT_TEMPLATE)
        );
    }
}

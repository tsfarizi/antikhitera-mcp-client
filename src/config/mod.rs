use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info};
use utoipa::ToSchema;

const DEFAULT_MODEL: &str = "llama3";
const DEFAULT_CONFIG_PATH: &str = "config/client.toml";
pub const CONFIG_PATH: &str = DEFAULT_CONFIG_PATH;
pub const DEFAULT_PROMPT_TEMPLATE: &str = r#"
Anda adalah petugas Pelayanan Publik Kelurahan Cakung Barat. Layani warga dengan ramah, gunakan bahasa yang sopan, dan berikan langkah konkret yang dapat mereka lakukan.

{{custom_instruction}}

{{language_guidance}}

{{tool_guidance}}

Selalu ringkas informasi penting dalam bentuk daftar bila diperlukan dan pastikan warga memahami langkah selanjutnya.
"#;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub model: String,
    pub system_prompt: Option<String>,
    pub tools: Vec<ToolConfig>,
    pub prompt_template: Option<String>,
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
struct RawConfig {
    model: Option<String>,
    system_prompt: Option<String>,
    #[serde(default)]
    tools: Vec<RawTool>,
    prompt_template: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub struct ToolConfig {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawTool {
    Name(String),
    Detailed {
        name: String,
        description: Option<String>,
    },
}

impl AppConfig {
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
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
            model: DEFAULT_MODEL.to_string(),
            system_prompt: None,
            tools: Vec::new(),
            prompt_template: Some(DEFAULT_PROMPT_TEMPLATE.to_string()),
        }
    }
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
    Ok(AppConfig {
        model: parsed.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        system_prompt: parsed.system_prompt,
        tools: parsed.tools.into_iter().map(ToolConfig::from).collect(),
        prompt_template: Some(
            parsed
                .prompt_template
                .unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string()),
        ),
    })
}

impl From<RawTool> for ToolConfig {
    fn from(value: RawTool) -> Self {
        match value {
            RawTool::Name(name) => Self {
                name,
                description: None,
            },
            RawTool::Detailed { name, description } => Self { name, description },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
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
        assert!(config.system_prompt.is_none());
        assert!(config.tools.is_empty());
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
        assert_eq!(config.system_prompt.as_deref(), Some("keep short"));
        assert!(config.tools.is_empty());
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
        assert_eq!(config.system_prompt.as_deref(), Some("only system"));
        assert!(config.tools.is_empty());
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
        assert_eq!(config.tools[1].name, "tool-b");
        assert_eq!(config.tools[1].description.as_deref(), Some("Second tool"));
        assert_eq!(
            config.prompt_template.as_deref(),
            Some(DEFAULT_PROMPT_TEMPLATE)
        );
    }
}

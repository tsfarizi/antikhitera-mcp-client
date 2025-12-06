use serde::Deserialize;
use shellexpand;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    pub name: String,
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub workdir: Option<PathBuf>,
    pub default_timezone: Option<String>,
    pub default_city: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawServer {
    name: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    workdir: Option<String>,
    #[serde(default)]
    default_timezone: Option<String>,
    #[serde(default)]
    default_city: Option<String>,
}

impl From<RawServer> for ServerConfig {
    fn from(raw: RawServer) -> Self {
        let expand = |s: &str| -> String {
            shellexpand::full(s)
                .map(|cow| cow.into_owned())
                .unwrap_or_else(|_| s.to_string())
        };

        let command_str = expand(&raw.command);
        let command = PathBuf::from(command_str);

        let workdir = raw.workdir.map(|d| PathBuf::from(expand(&d)));

        let args = raw.args.into_iter().map(|arg| expand(&arg)).collect();

        Self {
            name: raw.name,
            command,
            args,
            env: raw.env,
            workdir,
            default_timezone: raw.default_timezone,
            default_city: raw.default_city,
        }
    }
}

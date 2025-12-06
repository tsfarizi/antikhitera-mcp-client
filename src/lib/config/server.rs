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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn expands_env_vars_in_command_and_args() {
        unsafe {
            env::set_var("TEST_MCP_ROOT", "/path/to/mcp");
            env::set_var("TEST_ARG", "example-arg");
        }

        let raw = RawServer {
            name: "test-server".to_string(),
            command: "${TEST_MCP_ROOT}/server".to_string(),
            args: vec!["--flag", "${TEST_ARG}"]
                .into_iter()
                .map(String::from)
                .collect(),
            env: HashMap::new(),
            workdir: Some("${TEST_MCP_ROOT}/work".to_string()),
            default_timezone: None,
            default_city: None,
        };

        let config = ServerConfig::from(raw);

        // Verification logic
        let cmd = config.command.to_str().expect("valid utf8");
        // On Windows it might be \, on Linux /. shellexpand uses / by default or system separator?
        // shellexpand::full uses system env vars.
        // We check if it contains the expanded value.
        // Note: paths might be mixed on Windows (forward/backward slashes), checking strict equality might be flaky if separator differs.
        // But here we expect simple string substitution.

        assert!(cmd.contains("/path/to/mcp/server") || cmd.contains("\\path\\to\\mcp\\server"));
        assert!(config.args.contains(&"example-arg".to_string()));

        let workdir = config.workdir.expect("workdir exists");
        let workdir_str = workdir.to_str().expect("valid utf8");
        assert!(
            workdir_str.contains("/path/to/mcp/work")
                || workdir_str.contains("\\path\\to\\mcp\\work")
        );

        unsafe {
            env::remove_var("TEST_MCP_ROOT");
            env::remove_var("TEST_ARG");
        }
    }
}

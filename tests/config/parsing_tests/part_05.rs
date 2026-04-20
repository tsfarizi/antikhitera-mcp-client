#[test]
fn parses_servers_and_tools() {
    let dir = tempdir().expect("tempdir");
    let client_content = r#"
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
"#;

    let model_content = r#"
model = "mistral"
default_provider = "ollama"
prompt_template = "Be helpful."

[[tools]]
name = "get_time"
description = "Fetch time"
server = "time"
"#;

    let path = write_configs(
        dir.path(),
        client_content,
        model_content,
        minimal_ui_config(),
    );
    let config = AppConfig::load(Some(&path)).expect("load config");

    assert_eq!(config.servers.len(), 2);
    assert_eq!(config.servers[0].name, "time");
    assert_eq!(config.servers[0].args, vec!["--flag"]);
    assert_eq!(
        config.servers[0].default_timezone.as_deref(),
        Some("Asia/Jakarta")
    );

    assert_eq!(config.tools.len(), 1);
    assert_eq!(config.tools[0].name, "get_time");
    assert_eq!(config.tools[0].server.as_deref(), Some("time"));
}

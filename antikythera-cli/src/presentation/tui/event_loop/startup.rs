use std::collections::HashMap;
use std::sync::Arc;

use antikythera_core::config::AppConfig;

pub(super) async fn bootstrap_servers_and_transports(
    config: &mut AppConfig,
) -> (
    Option<String>,
    HashMap<String, Arc<antikythera_core::application::tooling::BuiltinTransport>>,
) {
    // ── Server auto-discovery ─────────────────────────────────────────────────
    let discovery_msg = {
        use antikythera_core::application::discovery::startup::run_startup_discovery;
        use antikythera_core::config::server::{ServerConfig as CoreServerConfig, TransportType};

        let result = run_startup_discovery(None).await;
        let mut added = 0usize;
        for server in result.loaded_servers() {
            let sc = CoreServerConfig {
                name: server.name.clone(),
                transport: TransportType::Stdio,
                command: Some(server.binary_path.clone()),
                args: Vec::new(),
                env: HashMap::new(),
                workdir: None,
                url: None,
                headers: HashMap::new(),
                default_timezone: None,
                default_city: None,
            };
            if !config.servers.iter().any(|s| s.name == sc.name) {
                config.servers.push(sc);
                added += 1;
            }
        }
        if result.folder_exists {
            Some(format!(
                "Server discovery: {} ditemukan, {} berhasil diload, {} ditambahkan ke konfigurasi aktif.",
                result.summary.total_found, result.summary.loaded, added
            ))
        } else {
            None
        }
    };

    // ── Builtin MCP transport registration ────────────────────────────────────
    let builtin_transports = {
        use antikythera_core::application::tooling::{
            BuiltinTransport, ServerToolInfo, TaskSupport, ToolAnnotations, ToolExecution,
        };
        use antikythera_core::config::server::{ServerConfig as CoreServerConfig, TransportType};
        use antikythera_core::config::tool::ToolConfig;

        let builtin_server_name = "builtin_time";
        if !config.servers.iter().any(|s| s.name == builtin_server_name) {
            config.servers.push(CoreServerConfig {
                name: builtin_server_name.to_string(),
                transport: TransportType::Builtin,
                command: None,
                args: Vec::new(),
                env: HashMap::new(),
                workdir: None,
                url: None,
                headers: HashMap::new(),
                default_timezone: None,
                default_city: None,
            });
        }

        let builtin_tools = [("get_current_date", "Get today's date in dd mm yyyy format")];
        for (tool_name, tool_desc) in builtin_tools {
            if !config.tools.iter().any(|t| t.name == tool_name) {
                config.tools.push(ToolConfig {
                    name: tool_name.to_string(),
                    description: Some(tool_desc.to_string()),
                    server: Some(builtin_server_name.to_string()),
                });
            }
        }

        let tool_infos = vec![ServerToolInfo {
            name: "get_current_date".to_string(),
            title: Some("Current Date Provider".to_string()),
            description: Some(
                "Get today's date in dd mm yyyy format. Returns the current date based on the system clock."
                    .to_string(),
            ),
            icons: None,
            input_schema: Some(serde_json::json!({
                "type": "object",
                "additionalProperties": false
            })),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "Today's date in dd mm yyyy format" },
                    "day": { "type": "string" },
                    "month": { "type": "string" },
                    "year": { "type": "string" }
                },
                "required": ["date"]
            })),
            annotations: Some(ToolAnnotations {
                audience: Some(vec!["user".to_string(), "assistant".to_string()]),
                priority: Some(1.0),
                last_modified: None,
            }),
            execution: Some(ToolExecution {
                task_support: Some(TaskSupport::Forbidden),
            }),
        }];

        let transport = BuiltinTransport::with_tools(builtin_server_name, tool_infos).with_handler(
            "get_current_date",
            |_args| {
                let now = chrono::Local::now();
                Ok(serde_json::json!({
                    "date": now.format("%d %m %Y").to_string(),
                    "day": now.format("%A").to_string(),
                    "month": now.format("%B").to_string(),
                    "year": now.format("%Y").to_string(),
                }))
            },
        );

        let mut map = HashMap::new();
        map.insert(builtin_server_name.to_string(), Arc::new(transport));
        map
    };

    // ── Prompt template customisation ─────────────────────────────────────────
    config.prompts.template = Some(
        concat!(
            "You are a helpful AI assistant.\n\n",
            // --- Format rules first so the model sees them immediately ---
            "CRITICAL: Your ENTIRE response must be exactly ONE of the two JSON objects below.\n",
            "Do NOT return plain text. Do NOT wrap in markdown. Do NOT add commentary.\n\n",

            "=== TOOL CALL (to use a tool) ===\n",
            r#"{"action":"call_tool","tool":"TOOL_NAME","input":{...}}"#,
            "\n",
            "Example: ",
            r#"{"action":"call_tool","tool":"get_current_date","input":{}}"#,
            "\n",
            r#"WRONG: "tool_call", "tool_calls", "function", "parameters", "arguments" — all rejected."#,
            "\n\n",

            "=== FINAL RESPONSE (to answer the user) ===\n",
            r#"{"action":"final","response":{"content":"your answer here"}}"#,
            "\n",
            "IMPORTANT: The ENTIRE response must be the JSON above. Do NOT return just ",
            r#"{"content":"..."} "#,
            "without the action/response wrapper.\n",
            "Only include a ",
            r#""data" "#,
            "field if you used a tool — reference the step like ",
            r#""data":"step_0""#,
            ". If no tool was used, omit the data field entirely.\n\n",

            "{{custom_instruction}}\n\n",
            "{{language_guidance}}\n\n",
            "{{tool_guidance}}",
        ).to_string(),
    );

    (discovery_msg, builtin_transports)
}

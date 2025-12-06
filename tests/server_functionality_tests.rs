// Server functionality tests - verifying MCP server communication
//
// Tests that spawn configured servers and verify JSON-RPC communication.

use antikhitera_mcp_client::config::{AppConfig, ServerConfig};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

// ============================================================================
// Test Client
// ============================================================================

const PROTOCOL_VERSION: &str = "2025-06-18";

struct McpTestClient {
    child: Child,
    request_id: u32,
    reader: BufReader<std::process::ChildStdout>,
}

impl McpTestClient {
    fn spawn(server: &ServerConfig) -> Result<Self, String> {
        let command_path = &server.command;
        if !command_path.exists() {
            return Err(format!(
                "Server executable not found: {}",
                command_path.display()
            ));
        }

        let mut cmd = Command::new(command_path);
        cmd.args(&server.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        if let Some(workdir) = &server.workdir {
            cmd.current_dir(workdir);
        }

        for (key, value) in &server.env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn: {}", e))?;
        let stdout = child.stdout.take().ok_or("No stdout")?;

        Ok(Self {
            child,
            request_id: 0,
            reader: BufReader::new(stdout),
        })
    }

    fn send_request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.request_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        });

        let stdin = self.child.stdin.as_mut().ok_or("No stdin")?;
        let mut line = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        line.push('\n');
        stdin
            .write_all(line.as_bytes())
            .map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;

        self.read_json_response()
    }

    fn read_json_response(&mut self) -> Result<Value, String> {
        for _ in 0..50 {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return Err("EOF".to_string()),
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('\x1b') {
                        continue;
                    }
                    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                        return Ok(value);
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("Timeout".to_string())
    }

    fn send_notification(&mut self, method: &str, params: Value) -> Result<(), String> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let stdin = self.child.stdin.as_mut().ok_or("No stdin")?;
        let mut line = serde_json::to_string(&notification).map_err(|e| e.to_string())?;
        line.push('\n');
        stdin
            .write_all(line.as_bytes())
            .map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn initialize(&mut self) -> Result<Value, String> {
        let response = self.send_request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0.1.0" }
            }),
        )?;
        self.send_notification("notifications/initialized", json!({}))?;
        Ok(response)
    }

    fn list_tools(&mut self) -> Result<Value, String> {
        self.send_request("tools/list", json!({}))
    }
}

impl Drop for McpTestClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ============================================================================
// Server Test Macro
// ============================================================================

macro_rules! server_test {
    ($test_name:ident, $server_name:expr) => {
        #[test]
        fn $test_name() {
            let config = AppConfig::load(Some(Path::new("config/client.toml")))
                .expect("Failed to load config");

            let server = config
                .servers
                .iter()
                .find(|s| s.name == $server_name)
                .expect(&format!("Server '{}' not found", $server_name));

            if !server.command.exists() {
                eprintln!("SKIPPED: {} not found", server.command.display());
                return;
            }

            let mut client =
                McpTestClient::spawn(server).expect(&format!("Failed to spawn '{}'", $server_name));

            thread::sleep(Duration::from_millis(100));

            // Test initialize
            let init = client.initialize().expect("Initialize failed");
            assert!(init.get("result").is_some() || init.get("error").is_none());

            // Test list tools
            let tools = client.list_tools().expect("List tools failed");
            if let Some(result) = tools.get("result") {
                if let Some(t) = result.get("tools") {
                    println!(
                        "✓ '{}' provides {} tools",
                        $server_name,
                        t.as_array().map(|a| a.len()).unwrap_or(0)
                    );
                }
            }
        }
    };
}

// ============================================================================
// Generated Tests
// ============================================================================

server_test!(time_server_responds_to_initialize, "time");
server_test!(certificate_server_responds_to_initialize, "certificate");

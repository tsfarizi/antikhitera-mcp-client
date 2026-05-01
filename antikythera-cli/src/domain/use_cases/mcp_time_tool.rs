//! MCP Time Tool — CLI Host-side Implementation
//!
//! Fungsionalitas `mcp_get_current_time` didefinisikan di sini sebagai
//! **host-side tool handler**. Tool ini didaftarkan ke WASM runner via
//! `register_tools` FFI, sehingga WASM agent dapat memanggilnya secara
//! dinamis. CLI host yang mengeksekusi logika aktualnya — bukan SDK/core.
//!
//! Pola ini menjaga core/SDK tetap agnostik: WASM hanya tahu nama dan
//! skema tool-nya, sedangkan implementasinya sepenuhnya milik host (CLI).

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::error::{CliError, CliResult};

// ============================================================================
// Tool Definition (didaftarkan ke WASM runner)
// ============================================================================

/// Kembalikan definisi tool JSON yang kompatibel dengan skema `register_tools` SDK.
pub fn mcp_time_tool_definition() -> Value {
    json!({
        "name": "mcp_get_current_time",
        "description": "Mengembalikan waktu Unix saat ini (detik sejak epoch) dari host.",
        "parameters": []
    })
}

// ============================================================================
// Host Executor
// ============================================================================

/// Eksekusi tool `mcp_get_current_time` di sisi host (CLI).
///
/// Dipanggil oleh CLI host ketika WASM agent melakukan `call_tool` dengan
/// nama `mcp_get_current_time`. Mengembalikan JSON output sesuai kontrak
/// `ToolExecutionResult.output_json`.
pub fn execute_mcp_get_current_time() -> CliResult<Value> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CliError::Validation(format!("system clock error: {e}")))?
        .as_secs();

    Ok(json!({
        "status": "success",
        "time": secs
    }))
}

/// Dispatch tool call dari WASM agent berdasarkan nama tool.
///
/// Kembalikan `Ok(output_json_string)` agar dapat langsung diumpankan
/// ke `process_tool_result` FFI.
pub fn dispatch_mcp_tool(tool_name: &str, _arguments: &Value) -> CliResult<String> {
    match tool_name {
        "mcp_get_current_time" => {
            let result = execute_mcp_get_current_time()?;
            serde_json::to_string(&result).map_err(CliError::Serialization)
        }
        other => Err(CliError::Validation(format!("unknown MCP tool: '{other}'"))),
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definition_has_correct_name() {
        let def = mcp_time_tool_definition();
        assert_eq!(def["name"], "mcp_get_current_time");
        assert!(def["description"].as_str().is_some());
        assert!(def["parameters"].as_array().is_some());
    }

    #[test]
    fn execute_returns_success_with_unix_time() {
        let result = execute_mcp_get_current_time().expect("should succeed");
        assert_eq!(result["status"], "success");
        let time = result["time"].as_u64().expect("time must be u64");
        assert!(time > 1_700_000_000, "unix time looks wrong: {time}");
    }

    #[test]
    fn dispatch_known_tool_returns_json_string() {
        let output =
            dispatch_mcp_tool("mcp_get_current_time", &json!({})).expect("dispatch should succeed");
        let parsed: Value = serde_json::from_str(&output).expect("output must be valid JSON");
        assert_eq!(parsed["status"], "success");
        assert!(parsed["time"].is_number());
    }

    #[test]
    fn dispatch_unknown_tool_returns_error() {
        let err = dispatch_mcp_tool("unknown_tool", &json!({}));
        assert!(err.is_err());
        let msg = format!("{:?}", err.unwrap_err());
        assert!(msg.contains("unknown MCP tool"));
    }
}

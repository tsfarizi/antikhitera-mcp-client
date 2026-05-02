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

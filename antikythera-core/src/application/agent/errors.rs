use crate::application::client::McpError;
use crate::application::tooling::ToolInvokeError;
use super::memory::MemoryError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error(transparent)]
    Client(#[from] McpError),
    #[error(transparent)]
    Tool(#[from] ToolError),
    #[error("invalid agent response: {0}")]
    InvalidResponse(String),
    #[error("maximum steps exceeded")]
    MaxStepsExceeded,
    #[error("operation timed out")]
    Timeout,
    #[error("memory error: {0}")]
    MemoryError(#[from] MemoryError),
}

impl AgentError {
    pub fn user_message(&self) -> String {
        match self {
            AgentError::Client(err) => err.user_message(),
            AgentError::Tool(err) => err.user_message(),
            AgentError::InvalidResponse(msg) => {
                format!("AI memberikan respons yang tidak dapat dipahami. Coba ulangi instruksi Anda. Error: {}", msg)
            }
            AgentError::MaxStepsExceeded => {
                "Langkah maksimum terlampaui. Proses dihentikan.".to_string()
            }
            AgentError::Timeout => {
                "Operasi timeout. Silakan coba lagi.".to_string()
            }
            AgentError::MemoryError(err) => {
                format!("Error penyimpanan state: {}", err)
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("unknown tool requested: {0}")]
    UnknownTool(String),
    #[error("tool '{0}' is not bound to any MCP server")]
    UnboundTool(String),
    #[error("failed to execute tool '{tool}': {source}")]
    Execution {
        tool: String,
        #[source]
        source: ToolInvokeError,
    },
}

impl ToolError {
    pub fn user_message(&self) -> String {
        match self {
            ToolError::UnknownTool(name) => {
                format!("Tool \"{name}\" belum tersedia di server.")
            }
            ToolError::UnboundTool(name) => {
                format!(
                    "Tool \"{name}\" belum terhubung ke MCP server apa pun. Mohon periksa konfigurasi client."
                )
            }
            ToolError::Execution { tool, source } => {
                format!(
                    "Eksekusi tool \"{tool}\" gagal: {message}",
                    message = source.to_string()
                )
            }
        }
    }
}

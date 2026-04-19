mod error;
mod envelope;
mod interface;
mod manager;
#[cfg(feature = "native-transport")]
mod process;
pub mod transport;

pub use error::ToolInvokeError;
pub use envelope::{
	EnvelopeError, ToolCallEnvelope, ToolResultEnvelope,
	validate_tool_call_envelope, validate_tool_result_envelope,
};
pub use interface::{ServerToolInfo, ToolServerInterface};
pub use manager::ServerManager;
#[cfg(feature = "native-transport")]
pub use process::spawn_and_list_tools;
pub use transport::{HttpTransport, HttpTransportConfig, McpTransport, TransportMode};

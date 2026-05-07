mod envelope;
mod error;
mod interface;
#[cfg(feature = "native-transport")]
mod jsonrpc_client;
mod manager;
#[cfg(feature = "native-transport")]
mod process;
#[cfg(feature = "native-transport")]
mod tool_catalogue;
pub mod transport;

pub use envelope::{
    EnvelopeError, ToolCallEnvelope, ToolResultEnvelope, validate_tool_call_envelope,
    validate_tool_result_envelope,
};
pub use error::ToolInvokeError;
pub use interface::{
    PROTOCOL_VERSION, ServerToolInfo, TaskSupport, ToolAnnotations, ToolExecution, ToolIcon,
    ToolServerInterface,
};
pub use manager::ServerManager;
#[cfg(feature = "native-transport")]
pub use tool_catalogue::spawn_and_list_tools;
pub use transport::{
    BuiltinTransport, HttpTransport, HttpTransportConfig, McpTransport, TransportMode,
};

//! # Antikythera SDK
//!
//! High-level API wrapper with FFI and WASM bindings for the MCP client.
//!
//! ## Feature Flags
//!
//! - `wasm` - Enable WASM bindings (enabled by default)
//! - `ffi` - Enable FFI support for C bindings
//! - `single-agent` - Single agent support (default)
//! - `multi-agent` - Multi-agent orchestration support
//! - `cloud` - Cloud integrations (GCP)
//! - `wasm-sandbox` - WASM sandboxed tool execution
//! - `full` - All features (large binary, not recommended for WASM)
//!
//! ## Examples
//!
//! ### Minimal WASM build (single agent only)
//! ```bash
//! cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release
//! ```
//!
//! ### Multi-agent WASM build
//! ```bash
//! cargo build -p antikythera-sdk --target wasm32-unknown-unknown --release --no-default-features --features wasm,multi-agent
//! ```
//!
//! ### FFI build (native library)
//! ```bash
//! cargo build -p antikythera-sdk --release --features ffi
//! ```

// Re-export core types (always available)
pub use antikythera_core::application::agent::{Agent, AgentOptions, AgentOutcome};
pub use antikythera_core::application::client::{ClientConfig, McpClient};
pub use antikythera_core::config::AppConfig;

// Conditional exports based on features
#[cfg(feature = "multi-agent")]
pub use antikythera_core::application::agent::multi_agent::{
    AgentRegistry, AgentProfile, AgentRole, MemoryProvider, MemoryConfig, ContextId,
};

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod high_level_api;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the SDK (call once at startup)
pub fn init() {
    #[cfg(feature = "wasm")]
    wasm::init();
    
    #[cfg(not(feature = "wasm"))]
    console_println!("Antikythera SDK v{} initialized", VERSION);
}

#[cfg(not(feature = "wasm"))]
macro_rules! console_println {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(std::io::stdout(), $($arg)*);
        let _ = std::io::stdout().flush();
    }};
}

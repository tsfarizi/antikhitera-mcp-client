pub mod app;
pub mod error;
pub mod loader;
pub mod provider;
pub mod serializer;
pub mod server;
pub mod tool;
pub mod wizard;

/// Default config file path - can be overridden via CLI argument
pub const CONFIG_PATH: &str = "config/client.toml";

pub use app::AppConfig;
pub use error::ConfigError;
pub use provider::{ModelInfo, ModelProviderConfig};
pub use server::ServerConfig;
pub use tool::ToolConfig;

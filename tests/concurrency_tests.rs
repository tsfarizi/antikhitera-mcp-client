use antikythera_core::application::client::{ClientConfig, McpClient};
use antikythera_core::application::services::chat::ChatService;
use antikythera_core::config::AppConfig;
use antikythera_core::infrastructure::model::DynamicModelProvider;
use std::sync::Arc;
use std::time::Instant;
use tokio::task;

// Split into 5 parts for consistent test organization.
include!("concurrency_tests/part_01.rs");
include!("concurrency_tests/part_02.rs");
include!("concurrency_tests/part_03.rs");
include!("concurrency_tests/part_04.rs");
include!("concurrency_tests/part_05.rs");

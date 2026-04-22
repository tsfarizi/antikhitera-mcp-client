use antikythera_cli::config::load_app_config;
use antikythera_cli::infrastructure::llm::providers_from_postcard;
use antikythera_cli::runtime::build_runtime_client;
use antikythera_core::application::services::chat::ChatService;
use antikythera_core::config::AppConfig;
use std::sync::Arc;
use std::time::Instant;
use tokio::task;

// Split into 5 parts for consistent test organization.
include!("concurrency_tests/part_01.rs");
include!("concurrency_tests/part_02.rs");
include!("concurrency_tests/part_03.rs");
include!("concurrency_tests/part_04.rs");
include!("concurrency_tests/part_05.rs");

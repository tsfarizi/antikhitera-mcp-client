use antikythera_cli::config::load_app_config;
use antikythera_cli::infrastructure::llm::providers_from_postcard;
use antikythera_cli::runtime::build_runtime_client;
use antikythera_core::application::client::ChatRequest;
use antikythera_core::config::AppConfig;
use std::sync::Arc;
use std::time::Instant;
use tokio::task;

// Split into parts for consistent test organization.
include!("concurrency_tests/part_05.rs");

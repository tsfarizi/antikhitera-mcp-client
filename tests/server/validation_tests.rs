// Server validation tests - validating server configuration
//
// Tests that verify configuration references are valid.
// These tests gracefully skip if config files don't exist.

use antikythera_core::application::discovery::{
    load_server, scan_folder, DiscoveredServer, DiscoveryError, DiscoverySummary, LoadStatus,
    StartupDiscoveryResult, DEFAULT_SERVERS_FOLDER,
};
use antikythera_core::application::discovery::loader::create_server_config;
use antikythera_core::application::discovery::scanner::{extract_server_name, is_executable};
use antikythera_core::config::server::{RawServer, ServerConfig};
use antikythera_core::config::AppConfig;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

// Split into 11 parts for consistent test organization.
include!("validation_tests/part_01.rs");
include!("validation_tests/part_02.rs");
include!("validation_tests/part_03.rs");
include!("validation_tests/part_04.rs");
include!("validation_tests/part_05.rs");
include!("validation_tests/part_06.rs");
include!("validation_tests/part_07.rs");
include!("validation_tests/part_08.rs");
include!("validation_tests/part_09.rs");
include!("validation_tests/part_10.rs");
include!("validation_tests/part_11.rs");

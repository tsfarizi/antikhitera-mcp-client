// Transport tests - verifying HTTP transport and MCP transport abstraction
//
// Tests for HTTP transport configuration and JSON-RPC over HTTP.

mod http_transport_tests {
    use antikythera_core::config::{ServerConfig, TransportType};
    use antikythera_core::tooling::transport::{
        HttpTransport, HttpTransportConfig, TransportMode,
    };
    use std::collections::HashMap;

// Split into 5 parts for consistent test organization.
include!("transport_tests/part_01.rs");
include!("transport_tests/part_02.rs");
include!("transport_tests/part_03.rs");
include!("transport_tests/part_04.rs");
include!("transport_tests/part_05.rs");

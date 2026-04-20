use std::collections::HashMap;

use antikythera_core::{
    AuditCategory, AuditRecord, AuditTrail, InMemoryMetricsExporter, InMemoryTracingHook,
    LatencyTracker, MetricsExporter, TraceSpanContext, TraceStatus, TracingHook,
};

// Split into 5 parts for consistent test organization.
include!("observability_tests/part_01.rs");
include!("observability_tests/part_02.rs");
include!("observability_tests/part_03.rs");
include!("observability_tests/part_04.rs");
include!("observability_tests/part_05.rs");

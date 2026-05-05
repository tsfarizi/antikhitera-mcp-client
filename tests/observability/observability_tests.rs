use std::collections::HashMap;

use antikythera_core::{
    AuditCategory, AuditRecord, AuditTrail, CallerContext, InMemoryMetricsExporter,
    InMemoryObservabilityHook, InMemoryTracingHook, LatencyTracker, MetricKind, MetricsExporter,
    ObservabilityHook, TelemetryEvent, TraceSpanContext, TraceStatus, TracingHook,
};
use antikythera_core::application::observability::metrics::percentile;

include!("observability_tests/part_01.rs");
include!("observability_tests/part_02.rs");
include!("observability_tests/part_03.rs");
include!("observability_tests/part_04.rs");
include!("observability_tests/part_05.rs");
include!("observability_tests/part_06.rs");
include!("observability_tests/part_07.rs");
include!("observability_tests/part_08.rs");
include!("observability_tests/part_09.rs");

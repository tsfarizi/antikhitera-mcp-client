use std::collections::HashMap;

use antikythera_core::{
    AuditCategory, AuditRecord, AuditTrail, InMemoryMetricsExporter, InMemoryTracingHook,
    LatencyTracker, MetricsExporter, TraceSpanContext, TraceStatus, TracingHook,
};

#[test]
fn latency_tracker_computes_sla_percentiles() {
    let mut tracker = LatencyTracker::new();
    for value in [100.0, 120.0, 200.0, 220.0, 300.0] {
        tracker.record_ms(value);
    }

    let summary = tracker.summary();
    assert_eq!(summary.count, 5);
    assert_eq!(summary.p50_ms, 200.0);
    assert_eq!(summary.p95_ms, 300.0);
    assert_eq!(summary.p99_ms, 300.0);
}

#[test]
fn in_memory_metric_exporter_records_histogram_metrics() {
    let exporter = InMemoryMetricsExporter::new();
    let mut attributes = HashMap::new();
    attributes.insert("component".to_string(), "agent".to_string());

    exporter.export_histogram("latency.ms", 153.0, attributes);

    let snapshot = exporter.snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].name, "latency.ms");
}

#[test]
fn audit_trail_captures_policy_and_tool_events() {
    let trail = AuditTrail::new();
    trail.append(AuditRecord::new(
        AuditCategory::PolicyDecision,
        "allow:model:gpt-4",
        true,
        Some("corr-123".to_string()),
    ));
    trail.append(AuditRecord::new(
        AuditCategory::ToolExecution,
        "deny:tool:filesystem.write",
        false,
        Some("corr-123".to_string()),
    ));

    assert_eq!(trail.by_category(AuditCategory::PolicyDecision).len(), 1);
    assert_eq!(trail.by_category(AuditCategory::ToolExecution).len(), 1);
}

#[test]
fn tracing_hook_tracks_start_and_end_lifecycle() {
    let hook = InMemoryTracingHook::new();
    let span = TraceSpanContext::new("trace-a", "span-a", "model_request")
        .with_parent("root-span")
        .with_correlation_id("corr-a");

    hook.on_span_start(span.clone());
    hook.on_span_end(span.clone(), TraceStatus::Ok);

    assert_eq!(hook.started_spans().len(), 1);
    assert_eq!(hook.ended_spans().len(), 1);
    assert_eq!(hook.ended_spans()[0].1, TraceStatus::Ok);
}

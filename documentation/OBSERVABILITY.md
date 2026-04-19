# Observability and Metrics (v0.9.9 Priority 1)

This document covers the production observability surface added in v0.9.9
Priority 1.

## Scope

- SLA-oriented latency summaries (`p50`, `p95`, `p99`)
- Host-facing metric export hooks
- Structured audit trail records for policy and tool decisions
- Correlation ID propagation helpers
- Tracing hooks that can bridge to OpenTelemetry

All features are additive and backward-compatible.

## Core APIs

The following types are exported from `antikythera_core`:

- `CallerContext`
- `TelemetryEvent`
- `MetricsExporter`, `MetricRecord`, `MetricKind`
- `InMemoryMetricsExporter`
- `LatencyTracker`, `LatencySummary`
- `AuditTrail`, `AuditRecord`, `AuditCategory`
- `TracingHook`, `TraceSpanContext`, `TraceStatus`, `InMemoryTracingHook`

## Latency Summary (SLA)

Use `LatencyTracker` to collect operation latencies and derive percentile
metrics for monitoring dashboards and alerting.

```rust
use antikythera_core::LatencyTracker;

let mut tracker = LatencyTracker::new();
tracker.record_ms(100.0);
tracker.record_ms(160.0);
tracker.record_ms(220.0);

let summary = tracker.summary();
assert!(summary.p95_ms >= summary.p50_ms);
```

## Metrics Export Hooks

Implement `MetricsExporter` to bridge into your telemetry backend.

```rust
use std::collections::HashMap;
use antikythera_core::{InMemoryMetricsExporter, MetricsExporter};

let exporter = InMemoryMetricsExporter::new();
exporter.export_counter("tool.calls", 1.0, HashMap::new());
assert_eq!(exporter.snapshot().len(), 1);
```

## Audit Trails

`AuditTrail` stores structured records for compliance and post-incident review.

```rust
use antikythera_core::{AuditCategory, AuditRecord, AuditTrail};

let trail = AuditTrail::new();
trail.append(AuditRecord::new(
    AuditCategory::PolicyDecision,
    "allow:model:gpt-4",
    true,
    Some("corr-001".to_string()),
));

assert_eq!(trail.snapshot().len(), 1);
```

## Correlation Propagation

`CallerContext::ensure_correlation_id()` guarantees a correlation ID is
available before dispatching downstream operations.

## Tracing Hooks

`TracingHook` is the extension point for distributed tracing integration.

- `on_span_start(...)`
- `on_span_end(..., TraceStatus)`

Hosts can map these calls into OpenTelemetry spans or vendor SDKs.

## Tests

Coverage includes unit and integration tests for:

- percentile summaries and invalid sample handling
- metric export record capture
- audit trail filtering and details
- tracing span lifecycle hooks
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Metric type emitted by host-facing exporters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
}

/// Metric record captured by observability exporters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricRecord {
    pub name: String,
    pub kind: MetricKind,
    pub value: f64,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl MetricRecord {
    /// Construct a metric record.
    pub fn new(
        name: impl Into<String>,
        kind: MetricKind,
        value: f64,
        attributes: HashMap<String, String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            value,
            timestamp_ms: super::now_unix_ms(),
            attributes,
        }
    }
}

/// Export hook for metrics backends such as Prometheus, CloudWatch, Datadog,
/// or OpenTelemetry collectors.
pub trait MetricsExporter: Send + Sync {
    fn export_metric(&self, metric: MetricRecord);

    fn export_counter(&self, name: &str, value: f64, attributes: HashMap<String, String>) {
        self.export_metric(MetricRecord::new(
            name,
            MetricKind::Counter,
            value,
            attributes,
        ));
    }

    fn export_gauge(&self, name: &str, value: f64, attributes: HashMap<String, String>) {
        self.export_metric(MetricRecord::new(
            name,
            MetricKind::Gauge,
            value,
            attributes,
        ));
    }

    fn export_histogram(&self, name: &str, value: f64, attributes: HashMap<String, String>) {
        self.export_metric(MetricRecord::new(
            name,
            MetricKind::Histogram,
            value,
            attributes,
        ));
    }
}

/// In-memory metric exporter used by tests and embedded hosts.
#[derive(Debug, Clone, Default)]
pub struct InMemoryMetricsExporter {
    metrics: Arc<Mutex<Vec<MetricRecord>>>,
}

impl InMemoryMetricsExporter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Vec<MetricRecord> {
        self.metrics.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.metrics.lock().unwrap().clear();
    }
}

impl MetricsExporter for InMemoryMetricsExporter {
    fn export_metric(&self, metric: MetricRecord) {
        self.metrics.lock().unwrap().push(metric);
    }
}

/// SLA latency summary values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatencySummary {
    pub count: usize,
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl Default for LatencySummary {
    fn default() -> Self {
        Self {
            count: 0,
            min_ms: 0.0,
            max_ms: 0.0,
            avg_ms: 0.0,
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
        }
    }
}

/// In-memory latency tracker with percentile summary helpers.
#[derive(Debug, Clone, Default)]
pub struct LatencyTracker {
    samples_ms: Vec<f64>,
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_ms(&mut self, duration_ms: f64) {
        if duration_ms.is_finite() && duration_ms >= 0.0 {
            self.samples_ms.push(duration_ms);
        }
    }

    pub fn count(&self) -> usize {
        self.samples_ms.len()
    }

    pub fn summary(&self) -> LatencySummary {
        if self.samples_ms.is_empty() {
            return LatencySummary::default();
        }

        let mut sorted = self.samples_ms.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let count = sorted.len();
        let sum: f64 = sorted.iter().sum();

        LatencySummary {
            count,
            min_ms: sorted[0],
            max_ms: sorted[count - 1],
            avg_ms: sum / count as f64,
            p50_ms: percentile(&sorted, 0.50),
            p95_ms: percentile(&sorted, 0.95),
            p99_ms: percentile(&sorted, 0.99),
        }
    }
}

pub(super) fn percentile(sorted_samples: &[f64], q: f64) -> f64 {
    if sorted_samples.is_empty() {
        return 0.0;
    }

    let q = q.clamp(0.0, 1.0);
    let index = ((sorted_samples.len() - 1) as f64 * q).round() as usize;
    sorted_samples[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_metrics_exporter_collects_counter_records() {
        let exporter = InMemoryMetricsExporter::new();
        exporter.export_counter("tool.calls", 1.0, Default::default());
        exporter.export_counter("tool.calls", 2.0, Default::default());

        let snapshot = exporter.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert!(snapshot.iter().all(|r| r.kind == MetricKind::Counter));
    }

    #[test]
    fn metrics_exporter_clear_resets_records() {
        let exporter = InMemoryMetricsExporter::new();
        exporter.export_counter("x", 1.0, Default::default());
        exporter.clear();
        assert!(exporter.snapshot().is_empty());
    }

    #[test]
    fn latency_tracker_summary_reports_percentiles() {
        let mut tracker = LatencyTracker::new();
        tracker.record_ms(100.0);
        tracker.record_ms(200.0);
        tracker.record_ms(300.0);

        let summary = tracker.summary();
        assert_eq!(summary.count, 3);
        assert_eq!(summary.min_ms, 100.0);
        assert_eq!(summary.max_ms, 300.0);
        assert_eq!(summary.p50_ms, 200.0);
    }

    #[test]
    fn latency_tracker_ignores_negative_and_nan_values() {
        let mut tracker = LatencyTracker::new();
        tracker.record_ms(-10.0);
        tracker.record_ms(f64::NAN);
        tracker.record_ms(50.0);

        assert_eq!(tracker.count(), 1);
    }

    #[test]
    fn percentile_returns_zero_for_empty_samples() {
        assert_eq!(percentile(&[], 0.95), 0.0);
    }
}

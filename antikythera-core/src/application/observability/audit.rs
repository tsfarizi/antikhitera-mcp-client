use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Auditable event category for policy/tool observability trails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    PolicyDecision,
    ToolExecution,
    ModelRequest,
}

/// Structured audit record emitted by runtime checkpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub category: AuditCategory,
    pub action: String,
    pub allowed: bool,
    pub correlation_id: Option<String>,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub details: HashMap<String, String>,
}

impl AuditRecord {
    pub fn new(
        category: AuditCategory,
        action: impl Into<String>,
        allowed: bool,
        correlation_id: Option<String>,
    ) -> Self {
        Self {
            category,
            action: action.into(),
            allowed,
            correlation_id,
            timestamp_ms: super::now_unix_ms(),
            details: HashMap::new(),
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// In-memory audit trail store.
#[derive(Debug, Clone, Default)]
pub struct AuditTrail {
    records: Arc<Mutex<Vec<AuditRecord>>>,
}

impl AuditTrail {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&self, record: AuditRecord) {
        self.records.lock().unwrap().push(record);
    }

    pub fn snapshot(&self) -> Vec<AuditRecord> {
        self.records.lock().unwrap().clone()
    }

    pub fn by_category(&self, category: AuditCategory) -> Vec<AuditRecord> {
        self.snapshot()
            .into_iter()
            .filter(|record| record.category == category)
            .collect()
    }

    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_trail_can_filter_by_category() {
        let trail = AuditTrail::new();
        trail.append(AuditRecord::new(
            AuditCategory::PolicyDecision,
            "allow_model",
            true,
            Some("corr-1".to_string()),
        ));
        trail.append(AuditRecord::new(
            AuditCategory::ToolExecution,
            "invoke_tool",
            true,
            Some("corr-1".to_string()),
        ));

        let policies = trail.by_category(AuditCategory::PolicyDecision);
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].action, "allow_model");
    }

    #[test]
    fn audit_record_with_detail_sets_fields() {
        let record = AuditRecord::new(AuditCategory::ToolExecution, "call_weather", true, None)
            .with_detail("tool", "weather");

        assert_eq!(record.details.get("tool"), Some(&"weather".to_string()));
    }
}

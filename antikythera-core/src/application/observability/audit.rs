use crate::logging::TransportLogger;
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
        match self.records.lock() {
            Ok(mut guard) => guard.push(record),
            Err(e) => {
                TransportLogger::new("audit")
                    .warn(format!("AuditTrail records lock poisoned in append: {}", e));
            }
        }
    }

    pub fn snapshot(&self) -> Vec<AuditRecord> {
        self.records
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|e| {
                TransportLogger::new("audit").warn(format!(
                    "AuditTrail records lock poisoned in snapshot: {}",
                    e
                ));
                Vec::new()
            })
    }

    pub fn by_category(&self, category: AuditCategory) -> Vec<AuditRecord> {
        self.snapshot()
            .into_iter()
            .filter(|record| record.category == category)
            .collect()
    }

    pub fn clear(&self) {
        match self.records.lock() {
            Ok(mut guard) => guard.clear(),
            Err(e) => {
                TransportLogger::new("audit")
                    .warn(format!("AuditTrail records lock poisoned in clear: {}", e));
            }
        }
    }
}

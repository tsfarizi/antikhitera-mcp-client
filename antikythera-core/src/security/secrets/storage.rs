//! Secret Storage Logic

use crate::security::config::SecretMetadata;
use std::collections::HashMap;

/// Stored secret with metadata
#[derive(Debug, Clone)]
pub struct StoredSecret {
    pub value: String,
    pub metadata: SecretMetadata,
}

/// Secret storage backend
#[derive(Debug)]
pub enum SecretStorage {
    Memory {
        secrets: HashMap<String, Vec<StoredSecret>>,
    },
    File {
        secrets: HashMap<String, Vec<StoredSecret>>,
        #[allow(dead_code)]
        path: String,
    },
}

/// Secret rotation policy
#[derive(Debug, Clone)]
pub enum SecretRotationPolicy {
    /// Rotate based on time interval
    TimeBased { interval_hours: u32 },
    /// Rotate based on usage count
    UsageBased { max_uses: u32, current_uses: u32 },
    /// Manual rotation only
    Manual,
}

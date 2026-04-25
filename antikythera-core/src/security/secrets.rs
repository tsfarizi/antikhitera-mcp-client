//! Secrets Management
//!
//! Secure storage and rotation of secrets with encryption at rest.

use super::config::{SecretMetadata, SecretsConfig};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Secret manager for secure storage and rotation
pub struct SecretManager {
    config: SecretsConfig,
    storage: Arc<Mutex<SecretStorage>>,
    rotation_task: Option<std::thread::JoinHandle<()>>,
}

/// Secret storage backend
#[derive(Debug)]
enum SecretStorage {
    Memory {
        secrets: HashMap<String, Vec<StoredSecret>>,
    },
    File {
        secrets: HashMap<String, Vec<StoredSecret>>,
        #[allow(dead_code)]
        path: String,
    },
}

/// Stored secret with metadata
#[derive(Debug, Clone)]
struct StoredSecret {
    value: String,
    metadata: SecretMetadata,
}

/// Secret rotation policy
#[derive(Debug, Clone)]
pub enum SecretRotationPolicy {
    /// Rotate based on time interval
    TimeBased {
        interval_hours: u32,
    },
    /// Rotate based on usage count
    UsageBased {
        max_uses: u32,
        current_uses: u32,
    },
    /// Manual rotation only
    Manual,
}

/// Secret manager error
#[derive(Debug, Clone)]
pub enum SecretManagerError {
    SecretNotFound(String),
    SecretExpired(String),
    StorageError(String),
    EncryptionError(String),
    InvalidConfig(String),
}

impl std::fmt::Display for SecretManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretManagerError::SecretNotFound(id) => write!(f, "Secret not found: {}", id),
            SecretManagerError::SecretExpired(id) => write!(f, "Secret expired: {}", id),
            SecretManagerError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            SecretManagerError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            SecretManagerError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

impl std::error::Error for SecretManagerError {}

impl SecretManager {
    pub fn new(config: SecretsConfig) -> Result<Self, SecretManagerError> {
        let storage = match config.storage_backend.as_str() {
            "memory" => SecretStorage::Memory {
                secrets: HashMap::new(),
            },
            "file" => {
                let path = config.storage_path.clone().unwrap_or_else(|| ".secrets".to_string());
                SecretStorage::File {
                    secrets: HashMap::new(),
                    path,
                }
            }
            backend => {
                return Err(SecretManagerError::InvalidConfig(format!(
                    "Unknown storage backend: {}",
                    backend
                )))
            }
        };

        let storage = Arc::new(Mutex::new(storage));

        let rotation_task = if config.auto_rotate {
            let storage_clone = Arc::clone(&storage);
            let interval = Duration::from_secs((config.rotation_interval_hours * 3600) as u64);

            Some(std::thread::spawn(move || {
                Self::rotation_task(storage_clone, interval);
            }))
        } else {
            None
        };

        Ok(Self {
            config,
            storage,
            rotation_task,
        })
    }

    pub fn from_config() -> Result<Self, SecretManagerError> {
        Self::new(SecretsConfig::default())
    }

    /// Store a secret
    pub fn store_secret(&self, id: &str, value: &str) -> Result<(), SecretManagerError> {
        if !self.config.enabled {
            return Err(SecretManagerError::InvalidConfig("Secrets management is disabled".to_string()));
        }

        let encrypted_value = if self.config.encrypt_at_rest {
            self.encrypt(value)?
        } else {
            value.to_string()
        };

        let mut storage = self.storage.lock().unwrap();
        let secrets = match &mut *storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        let entry = secrets.entry(id.to_string()).or_insert_with(Vec::new);
        let version = entry.len() as u32 + 1;

        let metadata = SecretMetadata::new(id.to_string(), version);

        entry.push(StoredSecret {
            value: encrypted_value,
            metadata,
        });

        // Enforce max versions
        if self.config.enable_versioning && entry.len() as u32 > self.config.max_versions {
            entry.remove(0);
        }

        Ok(())
    }

    /// Retrieve a secret
    pub fn get_secret(&self, id: &str) -> Result<String, SecretManagerError> {
        let storage = self.storage.lock().unwrap();
        let secrets = match &*storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        let entry = secrets.get(id).ok_or_else(|| SecretManagerError::SecretNotFound(id.to_string()))?;

        // Get the latest active version
        let latest = entry
            .iter()
            .filter(|s| s.metadata.active)
            .max_by_key(|s| s.metadata.version)
            .ok_or_else(|| SecretManagerError::SecretNotFound(id.to_string()))?;

        if latest.metadata.is_expired() {
            return Err(SecretManagerError::SecretExpired(id.to_string()));
        }

        let value = if self.config.encrypt_at_rest {
            self.decrypt(&latest.value)?
        } else {
            latest.value.clone()
        };

        Ok(value)
    }

    /// Rotate a secret
    pub fn rotate_secret(&self, id: &str, new_value: &str) -> Result<(), SecretManagerError> {
        let mut storage = self.storage.lock().unwrap();
        let secrets = match &mut *storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        let entry = secrets.get_mut(id).ok_or_else(|| SecretManagerError::SecretNotFound(id.to_string()))?;

        // Deactivate old versions
        for secret in entry.iter_mut() {
            secret.metadata.active = false;
        }

        // Add new version
        let version = entry.len() as u32 + 1;
        let encrypted_value = if self.config.encrypt_at_rest {
            self.encrypt(new_value)?
        } else {
            new_value.to_string()
        };

        let mut metadata = SecretMetadata::new(id.to_string(), version);
        metadata.last_rotated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        entry.push(StoredSecret {
            value: encrypted_value,
            metadata,
        });

        // Enforce max versions
        if self.config.enable_versioning && entry.len() as u32 > self.config.max_versions {
            entry.remove(0);
        }

        Ok(())
    }

    /// Check if a secret needs rotation
    pub fn needs_rotation(&self, id: &str) -> Result<bool, SecretManagerError> {
        let storage = self.storage.lock().unwrap();
        let secrets = match &*storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        let entry = secrets.get(id).ok_or_else(|| SecretManagerError::SecretNotFound(id.to_string()))?;

        let latest = entry
            .iter()
            .filter(|s| s.metadata.active)
            .max_by_key(|s| s.metadata.version)
            .ok_or_else(|| SecretManagerError::SecretNotFound(id.to_string()))?;

        Ok(latest.metadata.needs_rotation(self.config.max_secret_age_hours))
    }

    /// Delete a secret
    pub fn delete_secret(&self, id: &str) -> Result<(), SecretManagerError> {
        let mut storage = self.storage.lock().unwrap();
        let secrets = match &mut *storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        secrets.remove(id).ok_or_else(|| SecretManagerError::SecretNotFound(id.to_string()))?;
        Ok(())
    }

    /// List all secret IDs
    pub fn list_secrets(&self) -> Vec<String> {
        let storage = self.storage.lock().unwrap();
        let secrets = match &*storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        secrets.keys().cloned().collect()
    }

    /// Get secret metadata
    pub fn get_metadata(&self, id: &str) -> Result<SecretMetadata, SecretManagerError> {
        let storage = self.storage.lock().unwrap();
        let secrets = match &*storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        let entry = secrets.get(id).ok_or_else(|| SecretManagerError::SecretNotFound(id.to_string()))?;

        let latest = entry
            .iter()
            .filter(|s| s.metadata.active)
            .max_by_key(|s| s.metadata.version)
            .ok_or_else(|| SecretManagerError::SecretNotFound(id.to_string()))?;

        Ok(latest.metadata.clone())
    }

    /// Encrypt a value (simplified implementation - use proper crypto in production)
    fn encrypt(&self, value: &str) -> Result<String, SecretManagerError> {
        // In production, use proper encryption like AES-256-GCM
        // This is a placeholder implementation
        Ok(format!("ENC:{}", value))
    }

    /// Decrypt a value (simplified implementation - use proper crypto in production)
    fn decrypt(&self, encrypted: &str) -> Result<String, SecretManagerError> {
        // In production, use proper decryption
        if encrypted.starts_with("ENC:") {
            Ok(encrypted[4..].to_string())
        } else {
            Err(SecretManagerError::EncryptionError("Invalid encrypted format".to_string()))
        }
    }

    /// Rotation task for automatic secret rotation
    fn rotation_task(storage: Arc<Mutex<SecretStorage>>, interval: Duration) {
        loop {
            std::thread::sleep(interval);

            let mut storage_guard = storage.lock().unwrap();
            let secrets = match &mut *storage_guard {
                SecretStorage::Memory { secrets } => secrets,
                SecretStorage::File { secrets, .. } => secrets,
            };

            // Check for secrets that need rotation
            for (_id, entry) in secrets.iter_mut() {
                if let Some(latest) = entry.iter_mut().filter(|s| s.metadata.active).max_by_key(|s| s.metadata.version) {
                    if latest.metadata.needs_rotation(720) { // 30 days default
                        // Mark for rotation (actual rotation would require new value)
                        latest.metadata.active = false;
                    }
                }
            }
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &SecretsConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: SecretsConfig) -> Result<(), SecretManagerError> {
        let rotation_interval_hours = config.rotation_interval_hours;
        let auto_rotate = config.auto_rotate;
        self.config = config;

        // Restart rotation task if enabled
        if auto_rotate && self.rotation_task.is_none() {
            let storage_clone = Arc::clone(&self.storage);
            let interval = Duration::from_secs((rotation_interval_hours * 3600) as u64);

            self.rotation_task = Some(std::thread::spawn(move || {
                Self::rotation_task(storage_clone, interval);
            }));
        }

        Ok(())
    }
}

impl Drop for SecretManager {
    fn drop(&mut self) {
        // Note: Thread cleanup is handled automatically when the thread completes
        // We don't explicitly abort threads as it can cause resource leaks
        self.rotation_task.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_get_secret() {
        let manager = SecretManager::from_config().unwrap();
        let id = "test-secret";
        let value = "my-secret-value";

        manager.store_secret(id, value).unwrap();
        let retrieved = manager.get_secret(id).unwrap();

        assert_eq!(retrieved, value);
    }

    #[test]
    fn test_rotate_secret() {
        let manager = SecretManager::from_config().unwrap();
        let id = "test-secret";
        let old_value = "old-value";
        let new_value = "new-value";

        manager.store_secret(id, old_value).unwrap();
        manager.rotate_secret(id, new_value).unwrap();

        let retrieved = manager.get_secret(id).unwrap();
        assert_eq!(retrieved, new_value);
    }

    #[test]
    fn test_delete_secret() {
        let manager = SecretManager::from_config().unwrap();
        let id = "test-secret";

        manager.store_secret(id, "value").unwrap();
        manager.delete_secret(id).unwrap();

        assert!(manager.get_secret(id).is_err());
    }

    #[test]
    fn test_list_secrets() {
        let manager = SecretManager::from_config().unwrap();

        manager.store_secret("secret1", "value1").unwrap();
        manager.store_secret("secret2", "value2").unwrap();

        let secrets = manager.list_secrets();
        assert_eq!(secrets.len(), 2);
        assert!(secrets.contains(&"secret1".to_string()));
        assert!(secrets.contains(&"secret2".to_string()));
    }

    #[test]
    fn test_get_metadata() {
        let manager = SecretManager::from_config().unwrap();
        let id = "test-secret";

        manager.store_secret(id, "value").unwrap();
        let metadata = manager.get_metadata(id).unwrap();

        assert_eq!(metadata.id, id);
        assert_eq!(metadata.version, 1);
        assert!(metadata.active);
    }
}
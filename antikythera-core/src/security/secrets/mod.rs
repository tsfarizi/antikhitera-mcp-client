//! Secrets Management
//!
//! Secure storage and rotation of secrets with encryption at rest.

pub mod crypto;
pub mod error;
pub mod storage;

use crypto::CryptoProvider;
pub use error::SecretManagerError;
pub use storage::{SecretRotationPolicy, SecretStorage, StoredSecret};

use crate::logging::SecurityLogger;
use crate::security::config::{SecretMetadata, SecretsConfig};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Secret manager for secure storage and rotation
pub struct SecretManager {
    config: SecretsConfig,
    log: SecurityLogger,
    storage: Arc<Mutex<SecretStorage>>,
    rotation_task: Option<std::thread::JoinHandle<()>>,
}

impl SecretManager {
    pub fn new(config: SecretsConfig) -> Result<Self, SecretManagerError> {
        let storage = match config.storage_backend.as_str() {
            "memory" => SecretStorage::Memory {
                secrets: HashMap::new(),
            },
            "file" => {
                let path = config
                    .storage_path
                    .clone()
                    .unwrap_or_else(|| ".secrets".to_string());
                SecretStorage::File {
                    secrets: HashMap::new(),
                    path,
                }
            }
            backend => {
                return Err(SecretManagerError::InvalidConfig(format!(
                    "Unknown storage backend: {}",
                    backend
                )));
            }
        };

        let storage = Arc::new(Mutex::new(storage));

        let rotation_task = if config.auto_rotate {
            let storage_clone = Arc::clone(&storage);
            let interval = Duration::from_secs((config.rotation_interval_hours * 3600) as u64);

            Some(std::thread::spawn(move || {
                Self::run_rotation_task(storage_clone, interval);
            }))
        } else {
            None
        };

        Ok(Self {
            config,
            log: SecurityLogger::new(&crate::logging::get_active_session()),
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
            self.log.secret_error(id, "secrets management is disabled");
            return Err(SecretManagerError::InvalidConfig(
                "Secrets management is disabled".to_string(),
            ));
        }

        let encrypted_value = if self.config.encrypt_at_rest {
            CryptoProvider::encrypt(value)?
        } else {
            value.to_string()
        };

        let mut storage = self
            .storage
            .lock()
            .expect("SecretManager storage lock poisoned in store_secret");
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

        self.log.secret_stored(id);
        Ok(())
    }

    /// Retrieve a secret
    pub fn get_secret(&self, id: &str) -> Result<String, SecretManagerError> {
        let storage = self
            .storage
            .lock()
            .expect("SecretManager storage lock poisoned in get_secret");
        let secrets = match &*storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        let entry = secrets.get(id).ok_or_else(|| {
            self.log.secret_error(id, "secret not found");
            SecretManagerError::SecretNotFound(id.to_string())
        })?;

        // Get the latest active version
        let latest = entry
            .iter()
            .filter(|s| s.metadata.active)
            .max_by_key(|s| s.metadata.version)
            .ok_or_else(|| {
                self.log.secret_error(id, "no active version found");
                SecretManagerError::SecretNotFound(id.to_string())
            })?;

        if latest.metadata.is_expired() {
            self.log.secret_error(id, "secret expired");
            return Err(SecretManagerError::SecretExpired(id.to_string()));
        }

        let value = if self.config.encrypt_at_rest {
            CryptoProvider::decrypt(&latest.value)?
        } else {
            latest.value.clone()
        };

        self.log.secret_retrieved(id);
        Ok(value)
    }

    /// Rotate a secret
    pub fn rotate_secret(&self, id: &str, new_value: &str) -> Result<(), SecretManagerError> {
        let mut storage = self
            .storage
            .lock()
            .expect("SecretManager storage lock poisoned in rotate_secret");
        let secrets = match &mut *storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        let entry = secrets.get_mut(id).ok_or_else(|| {
            self.log.secret_error(id, "secret not found for rotation");
            SecretManagerError::SecretNotFound(id.to_string())
        })?;

        // Deactivate old versions
        for secret in entry.iter_mut() {
            secret.metadata.active = false;
        }

        // Add new version
        let version = entry.len() as u32 + 1;
        let encrypted_value = if self.config.encrypt_at_rest {
            CryptoProvider::encrypt(new_value)?
        } else {
            new_value.to_string()
        };

        let mut metadata = SecretMetadata::new(id.to_string(), version);
        metadata.last_rotated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("SystemTime clock went backwards — cannot get duration since UNIX_EPOCH in rotate_secret")
            .as_secs();

        entry.push(StoredSecret {
            value: encrypted_value,
            metadata,
        });

        // Enforce max versions
        if self.config.enable_versioning && entry.len() as u32 > self.config.max_versions {
            entry.remove(0);
        }

        self.log.secret_rotated(id);
        Ok(())
    }

    /// Check if a secret needs rotation
    pub fn needs_rotation(&self, id: &str) -> Result<bool, SecretManagerError> {
        let storage = self
            .storage
            .lock()
            .expect("SecretManager storage lock poisoned in needs_rotation");
        let secrets = match &*storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        let entry = secrets.get(id).ok_or_else(|| {
            self.log
                .secret_error(id, "secret not found for rotation check");
            SecretManagerError::SecretNotFound(id.to_string())
        })?;

        let latest = entry
            .iter()
            .filter(|s| s.metadata.active)
            .max_by_key(|s| s.metadata.version)
            .ok_or_else(|| {
                self.log
                    .secret_error(id, "no active version for rotation check");
                SecretManagerError::SecretNotFound(id.to_string())
            })?;

        Ok(latest
            .metadata
            .needs_rotation(self.config.max_secret_age_hours))
    }

    /// Delete a secret
    pub fn delete_secret(&self, id: &str) -> Result<(), SecretManagerError> {
        let mut storage = self
            .storage
            .lock()
            .expect("SecretManager storage lock poisoned in delete_secret");
        let secrets = match &mut *storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        secrets.remove(id).ok_or_else(|| {
            self.log.secret_error(id, "secret not found for deletion");
            SecretManagerError::SecretNotFound(id.to_string())
        })?;
        self.log.secret_deleted(id);
        Ok(())
    }

    /// List all secret IDs
    pub fn list_secrets(&self) -> Vec<String> {
        let storage = self
            .storage
            .lock()
            .expect("SecretManager storage lock poisoned in list_secrets");
        let secrets = match &*storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        secrets.keys().cloned().collect()
    }

    /// Get secret metadata
    pub fn get_metadata(&self, id: &str) -> Result<SecretMetadata, SecretManagerError> {
        let storage = self
            .storage
            .lock()
            .expect("SecretManager storage lock poisoned in get_metadata");
        let secrets = match &*storage {
            SecretStorage::Memory { secrets } => secrets,
            SecretStorage::File { secrets, .. } => secrets,
        };

        let entry = secrets.get(id).ok_or_else(|| {
            self.log.secret_error(id, "secret not found for metadata");
            SecretManagerError::SecretNotFound(id.to_string())
        })?;

        let latest = entry
            .iter()
            .filter(|s| s.metadata.active)
            .max_by_key(|s| s.metadata.version)
            .ok_or_else(|| {
                self.log.secret_error(id, "no active version for metadata");
                SecretManagerError::SecretNotFound(id.to_string())
            })?;

        Ok(latest.metadata.clone())
    }

    /// Rotation task for automatic secret rotation
    fn run_rotation_task(storage: Arc<Mutex<SecretStorage>>, interval: Duration) {
        loop {
            std::thread::sleep(interval);

            let mut storage_guard = storage
                .lock()
                .expect("SecretManager rotation storage lock poisoned");
            let secrets = match &mut *storage_guard {
                SecretStorage::Memory { secrets } => secrets,
                SecretStorage::File { secrets, .. } => secrets,
            };

            // Check for secrets that need rotation
            for (_id, entry) in secrets.iter_mut() {
                if let Some(latest) = entry
                    .iter_mut()
                    .filter(|s| s.metadata.active)
                    .max_by_key(|s| s.metadata.version)
                    .filter(|s| s.metadata.needs_rotation(720))
                {
                    // Mark for rotation (actual rotation would require new value)
                    latest.metadata.active = false;
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
                Self::run_rotation_task(storage_clone, interval);
            }));
        }

        Ok(())
    }
}

impl Drop for SecretManager {
    fn drop(&mut self) {
        // Note: Thread cleanup is handled automatically when the thread completes
        self.rotation_task.take();
    }
}



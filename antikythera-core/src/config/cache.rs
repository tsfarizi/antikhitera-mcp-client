//! Postcard Binary Configuration Cache
//!
//! This module provides fast binary serialization for configuration files using Postcard.
//! 
//! ## Flow:
//! 1. First load: TOML → Postcard cache
//! 2. Subsequent loads: Postcard cache directly (faster)
//! 3. On config update: TOML → Postcard cache (re-generate)
//!
//! ## Benefits:
//! - ⚡ Faster load times (binary vs text parsing)
//! - 💾 Smaller storage (compact binary representation)
//! - 🔄 Schema evolution support (version migration)
//! - ✅ Integrity checking (schema version validation)

use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn, error};

/// Current schema version for migration support
pub const SCHEMA_VERSION: u32 = 1;

/// Postcard cache file extension
const POSTCARD_EXT: &str = ".postcard";

/// Cached configuration with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCache<T> {
    /// Schema version for compatibility checking
    pub schema_version: u32,
    /// Timestamp when cache was generated
    pub cached_at: u64,
    /// Source TOML file path (for validation)
    pub source_path: String,
    /// The actual configuration data
    pub data: T,
}

impl<T> ConfigCache<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone,
{
    /// Create a new config cache
    pub fn new(data: T, source_path: &str) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source_path: source_path.to_string(),
            data,
        }
    }

    /// Check if cache is still valid
    pub fn is_valid(&self) -> bool {
        self.schema_version == SCHEMA_VERSION
    }

    /// Get cache age in seconds
    pub fn age(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(self.cached_at)
    }
}

/// Configuration cache manager
pub struct ConfigCacheManager {
    /// Cache directory path
    cache_dir: PathBuf,
}

impl ConfigCacheManager {
    /// Create a new cache manager
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Get cache file path for a source file
    pub fn get_cache_path(&self, source_path: &Path) -> PathBuf {
        let file_name = source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("config");
        
        self.cache_dir.join(format!("{}{}", file_name, POSTCARD_EXT))
    }

    /// Check if cache exists and is valid
    pub fn cache_exists(&self, source_path: &Path) -> bool {
        let cache_path = self.get_cache_path(source_path);
        
        if !cache_path.exists() {
            return false;
        }

        // Try to read and validate schema version
        match fs::read(&cache_path) {
            Ok(bytes) => {
                if let Ok(cache) = from_bytes::<ConfigCache<serde_json::Value>>(&bytes) {
                    cache.is_valid()
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    /// Load configuration from Postcard cache
    pub fn load_from_cache<T>(&self, source_path: &Path) -> Result<T, ConfigCacheError>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone,
    {
        let cache_path = self.get_cache_path(source_path);
        
        debug!(
            path = %cache_path.display(),
            "Loading configuration from Postcard cache"
        );

        let bytes = fs::read(&cache_path)
            .map_err(|e| ConfigCacheError::IoError {
                path: cache_path.clone(),
                source: e,
            })?;

        let cache: ConfigCache<T> = from_bytes(&bytes)
            .map_err(ConfigCacheError::PostcardError)?;

        // Validate schema version
        if !cache.is_valid() {
            warn!(
                schema_version = cache.schema_version,
                expected = SCHEMA_VERSION,
                "Cache schema version mismatch, will regenerate"
            );
            return Err(ConfigCacheError::SchemaMismatch {
                cached: cache.schema_version,
                expected: SCHEMA_VERSION,
            });
        }

        // Validate source path matches
        if cache.source_path != source_path.to_string_lossy() {
            warn!(
                cached_source = %cache.source_path,
                actual_source = %source_path.display(),
                "Cache source path mismatch, will regenerate"
            );
            return Err(ConfigCacheError::SourceMismatch {
                cached: cache.source_path,
                actual: source_path.to_string_lossy().to_string(),
            });
        }

        info!(
            path = %cache_path.display(),
            age_seconds = cache.age(),
            "Loaded configuration from Postcard cache"
        );

        Ok(cache.data)
    }

    /// Save configuration to Postcard cache
    pub fn save_to_cache<T>(&self, data: T, source_path: &Path) -> Result<(), ConfigCacheError>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone,
    {
        // Ensure cache directory exists
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| ConfigCacheError::IoError {
                path: self.cache_dir.clone(),
                source: e,
            })?;

        let cache_path = self.get_cache_path(source_path);
        let cache = ConfigCache::new(data, &source_path.to_string_lossy());

        debug!(
            path = %cache_path.display(),
            "Saving configuration to Postcard cache"
        );

        let bytes = to_allocvec(&cache)
            .map_err(ConfigCacheError::PostcardError)?;

        fs::write(&cache_path, &bytes)
            .map_err(|e| ConfigCacheError::IoError {
                path: cache_path.clone(),
                source: e,
            })?;

        info!(
            path = %cache_path.display(),
            size_bytes = bytes.len(),
            "Saved configuration to Postcard cache"
        );

        Ok(())
    }

    /// Invalidate cache (delete cache file)
    pub fn invalidate_cache(&self, source_path: &Path) -> Result<(), ConfigCacheError> {
        let cache_path = self.get_cache_path(source_path);
        
        if cache_path.exists() {
            fs::remove_file(&cache_path)
                .map_err(|e| ConfigCacheError::IoError {
                    path: cache_path.clone(),
                    source: e,
                })?;
            
            debug!(
                path = %cache_path.display(),
                "Invalidated configuration cache"
            );
        }

        Ok(())
    }

    /// Get cache file size in bytes
    pub fn get_cache_size(&self, source_path: &Path) -> Option<u64> {
        let cache_path = self.get_cache_path(source_path);
        fs::metadata(&cache_path).ok().map(|m| m.len())
    }
}

/// Configuration cache errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigCacheError {
    #[error("IO error for path {path}: {source}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Postcard serialization error: {0}")]
    PostcardError(#[from] postcard::Error),

    #[error("Schema version mismatch: cached={cached}, expected={expected}")]
    SchemaMismatch {
        cached: u32,
        expected: u32,
    },

    #[error("Source path mismatch: cached={cached}, actual={actual}")]
    SourceMismatch {
        cached: String,
        actual: String,
    },
}

/// Helper function to load config with cache support
pub fn load_config_with_cache<T, F>(
    cache_manager: &ConfigCacheManager,
    source_path: &Path,
    load_from_toml: F,
) -> Result<T, ConfigCacheError>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone,
    F: FnOnce() -> Result<T, ConfigCacheError>,
{
    // Try to load from cache first
    if cache_manager.cache_exists(source_path) {
        match cache_manager.load_from_cache::<T>(source_path) {
            Ok(config) => return Ok(config),
            Err(e) => {
                warn!(error = %e, "Cache load failed, will load from TOML");
                // Cache load failed, will load from TOML below
            }
        }
    }

    // Load from TOML source
    debug!(
        path = %source_path.display(),
        "Loading configuration from TOML source"
    );
    
    let config = load_from_toml()?;

    // Save to cache for next time
    if let Err(e) = cache_manager.save_to_cache(config.clone(), source_path) {
        warn!(error = %e, "Failed to save cache, will load from TOML next time");
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    #[test]
    fn test_cache_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let manager = ConfigCacheManager::new(temp_dir.path().to_path_buf());
        let source_path = Path::new("/test/config.toml");

        let config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };

        // Save to cache
        manager.save_to_cache(config.clone(), source_path).unwrap();

        // Load from cache
        let loaded: TestConfig = manager.load_from_cache(source_path).unwrap();

        assert_eq!(config, loaded);
    }

    #[test]
    fn test_cache_validation() {
        let temp_dir = tempdir().unwrap();
        let manager = ConfigCacheManager::new(temp_dir.path().to_path_buf());
        let source_path = Path::new("/test/config.toml");

        // Cache doesn't exist initially
        assert!(!manager.cache_exists(source_path));

        // Create cache
        let config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };
        manager.save_to_cache(config, source_path).unwrap();

        // Cache exists now
        assert!(manager.cache_exists(source_path));
    }

    #[test]
    fn test_cache_invalidation() {
        let temp_dir = tempdir().unwrap();
        let manager = ConfigCacheManager::new(temp_dir.path().to_path_buf());
        let source_path = Path::new("/test/config.toml");

        // Create cache
        let config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };
        manager.save_to_cache(config, source_path).unwrap();
        assert!(manager.cache_exists(source_path));

        // Invalidate cache
        manager.invalidate_cache(source_path).unwrap();
        assert!(!manager.cache_exists(source_path));
    }
}

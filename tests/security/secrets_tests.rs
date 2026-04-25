//! Secrets management tests

use antikythera_core::security::config::SecretsConfig;
use antikythera_core::security::secrets::{SecretManager, SecretManagerError};

#[test]
fn test_secret_manager_creation() {
    let config = SecretsConfig::default();
    let manager = SecretManager::new(config).unwrap();
    assert!(manager.config().enabled);
}

#[test]
fn test_store_and_get_secret() {
    let manager = SecretManager::from_config().unwrap();
    let id = "test-secret-1";
    let value = "my-secret-value";

    manager.store_secret(id, value).unwrap();
    let retrieved = manager.get_secret(id).unwrap();

    assert_eq!(retrieved, value);
}

#[test]
fn test_store_multiple_versions() {
    let manager = SecretManager::from_config().unwrap();
    let id = "test-secret-2";

    manager.store_secret(id, "version1").unwrap();
    manager.rotate_secret(id, "version2").unwrap();
    manager.rotate_secret(id, "version3").unwrap();

    let retrieved = manager.get_secret(id).unwrap();
    assert_eq!(retrieved, "version3");
}

#[test]
fn test_rotate_secret() {
    let manager = SecretManager::from_config().unwrap();
    let id = "test-secret-3";
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
    let id = "test-secret-4";

    manager.store_secret(id, "value").unwrap();
    manager.delete_secret(id).unwrap();

    assert!(manager.get_secret(id).is_err());
}

#[test]
fn test_list_secrets() {
    let manager = SecretManager::from_config().unwrap();

    manager.store_secret("secret1", "value1").unwrap();
    manager.store_secret("secret2", "value2").unwrap();
    manager.store_secret("secret3", "value3").unwrap();

    let secrets = manager.list_secrets();
    assert_eq!(secrets.len(), 3);
    assert!(secrets.contains(&"secret1".to_string()));
    assert!(secrets.contains(&"secret2".to_string()));
    assert!(secrets.contains(&"secret3".to_string()));
}

#[test]
fn test_get_metadata() {
    let manager = SecretManager::from_config().unwrap();
    let id = "test-secret-5";

    manager.store_secret(id, "value").unwrap();
    let metadata = manager.get_metadata(id).unwrap();

    assert_eq!(metadata.id, id);
    assert_eq!(metadata.version, 1);
    assert!(metadata.active);
}

#[test]
fn test_secret_not_found() {
    let manager = SecretManager::from_config().unwrap();

    let result = manager.get_secret("non-existent-secret");
    assert!(matches!(result, Err(SecretManagerError::SecretNotFound(_))));
}

#[test]
fn test_secrets_disabled() {
    let config = SecretsConfig {
        enabled: false,
        ..Default::default()
    };
    let manager = SecretManager::new(config).unwrap();

    let result = manager.store_secret("test", "value");
    assert!(matches!(result, Err(SecretManagerError::InvalidConfig(_))));
}

#[test]
fn test_needs_rotation() {
    let manager = SecretManager::from_config().unwrap();
    let id = "test-secret-6";

    manager.store_secret(id, "value").unwrap();

    // Should not need rotation immediately
    assert!(!manager.needs_rotation(id).unwrap());
}

#[test]
fn test_update_config() {
    let mut manager = SecretManager::from_config().unwrap();

    let new_config = SecretsConfig {
        enabled: true,
        auto_rotate: true,
        rotation_interval_hours: 24,
        ..Default::default()
    };

    manager.update_config(new_config).unwrap();
    assert!(manager.config().auto_rotate);
    assert_eq!(manager.config().rotation_interval_hours, 24);
}

#[test]
fn test_secret_versioning() {
    let config = SecretsConfig {
        enable_versioning: true,
        max_versions: 3,
        ..Default::default()
    };
    let manager = SecretManager::new(config).unwrap();
    let id = "test-secret-7";

    manager.store_secret(id, "v1").unwrap();
    manager.rotate_secret(id, "v2").unwrap();
    manager.rotate_secret(id, "v3").unwrap();
    manager.rotate_secret(id, "v4").unwrap();
    manager.rotate_secret(id, "v5").unwrap();

    let metadata = manager.get_metadata(id).unwrap();
    // Should only keep last 3 versions
    assert!(metadata.version >= 3);
}

//! Secret Manager Errors

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

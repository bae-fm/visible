//! The core error surface.

/// An error from visible-core. Every public fallible operation returns this.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Database(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("sync error: {0}")]
    Sync(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<coven::database::DbError> for CoreError {
    fn from(e: coven::database::DbError) -> Self {
        CoreError::Database(e.to_string())
    }
}

impl From<coven::config::ConfigError> for CoreError {
    fn from(e: coven::config::ConfigError) -> Self {
        CoreError::Config(e.to_string())
    }
}

impl From<coven::keys::KeyError> for CoreError {
    fn from(e: coven::keys::KeyError) -> Self {
        CoreError::Keyring(e.to_string())
    }
}

impl From<coven::encryption::EncryptionError> for CoreError {
    fn from(e: coven::encryption::EncryptionError) -> Self {
        CoreError::Sync(e.to_string())
    }
}

impl From<coven::storage::cloud::CloudHomeError> for CoreError {
    fn from(e: coven::storage::cloud::CloudHomeError) -> Self {
        CoreError::Sync(e.to_string())
    }
}

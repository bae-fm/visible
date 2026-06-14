//! The uniffi boundary types: the records and error the generated Swift/Kotlin
//! see, and the conversions from visible-core's domain types. Type translation
//! only — no logic.

use visible_core::{CoreError, LibraryInfo, Node};

/// A node as the UI consumes it. No `position` — the bridge returns children
/// already ordered, so the UI iterates in order rather than re-sorting.
#[derive(uniffi::Record)]
pub struct BridgeNode {
    pub id: String,
    pub parent_id: Option<String>,
    /// The node's title, or `None` while it is untitled. The UI renders an
    /// "Untitled" fallback for `None`.
    pub name: Option<String>,
    pub image_id: Option<String>,
}

impl From<Node> for BridgeNode {
    fn from(node: Node) -> Self {
        Self {
            id: node.id,
            parent_id: node.parent_id,
            name: node.name,
            image_id: node.image_id,
        }
    }
}

/// A library for the picker.
#[derive(uniffi::Record)]
pub struct BridgeLibrary {
    pub id: String,
    pub name: String,
}

impl From<LibraryInfo> for BridgeLibrary {
    fn from(info: LibraryInfo) -> Self {
        Self {
            id: info.id,
            name: info.name,
        }
    }
}

/// The error surface the generated Swift/Kotlin throw and switch on.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum BridgeError {
    #[error("not found: {msg}")]
    NotFound { msg: String },
    #[error("database error: {msg}")]
    Database { msg: String },
    #[error("config error: {msg}")]
    Config { msg: String },
    #[error("internal error: {msg}")]
    Internal { msg: String },
}

impl From<CoreError> for BridgeError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::NotFound(msg) => BridgeError::NotFound { msg },
            CoreError::Database(msg) => BridgeError::Database { msg },
            CoreError::Config(msg) => BridgeError::Config { msg },
            CoreError::Io(msg) => BridgeError::Internal { msg },
            CoreError::Internal(msg) => BridgeError::Internal { msg },
        }
    }
}

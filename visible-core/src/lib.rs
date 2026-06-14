//! visible-core: the node-tree domain over coven.
//!
//! Everything the user owns is a node in one self-referential tree (the house
//! at the root). coven owns the SQLite connection, the on-disk layout, and the
//! per-library config + device id; visible-core owns the node domain and the
//! image files those nodes point at.

pub mod app;
pub mod blob_plan;
pub mod config;
pub mod error;
pub mod library;
pub mod node;
pub mod share;
pub mod sync;

pub use app::RunningApp;
pub use config::init_keyring;
pub use error::CoreError;
pub use library::LibraryInfo;
pub use node::{Inventory, Node, SearchHit};
pub use share::{join_shared_library, remove_library, restore_shared_library};
pub use sync::{Member, MemberRole, OutboxSnapshot, S3ConfigData, Sync, SyncStatusInfo};

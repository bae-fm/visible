//! visible-core: the node-tree domain over coven.
//!
//! Everything the user owns is a node in one self-referential tree (the house
//! at the root). coven owns the SQLite connection, the on-disk layout, and the
//! per-library config + device id; visible-core owns the node domain and the
//! image files those nodes point at.

pub mod app;
pub mod error;
pub mod library;
pub mod node;
pub mod paths;

pub use app::RunningApp;
pub use error::CoreError;
pub use library::LibraryInfo;
pub use node::{Inventory, Node};

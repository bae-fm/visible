//! Composition root: open a library's database and hand back a running
//! application — the tokio runtime that owns the connection thread's async work
//! and the live [`Inventory`].

use std::path::Path;

use coven::library_dir::LibraryDir;
use coven::sync::session::SyncedTable;
use coven::{Database, UpdatedAtStamper};

use crate::error::CoreError;
use crate::library::open_config;
use crate::node::{Inventory, SCHEMA};

/// A fully opened library: the tokio runtime the bridge blocks on for database
/// calls, and the live node-tree service.
pub struct RunningApp {
    pub runtime: tokio::runtime::Runtime,
    pub inventory: Inventory,
}

/// Open the coven database for one library and run the schema. coven owns the
/// connection: [`Database::open`] runs its bookkeeping migration, then the
/// `nodes` schema, seeds the `_updated_at` register off the rows on disk, and
/// hands back the non-optional stamper every node write binds. `nodes` is the
/// only host synced table; coven injects its own `item_keys`.
pub fn open_database(
    library_dir: &LibraryDir,
    device_id: String,
) -> Result<(Database, UpdatedAtStamper), CoreError> {
    Database::open(
        &library_dir.db_path(),
        vec![SyncedTable::new("nodes")],
        device_id,
        |conn| conn.execute_batch(SCHEMA).map_err(Into::into),
    )
    .map_err(Into::into)
}

/// Open `library_id` under `data_dir` and bring up its [`Inventory`].
///
/// Local-only: no sync manager, no encryption, no cloud config.
pub fn bootstrap(data_dir: &Path, library_id: String) -> Result<RunningApp, CoreError> {
    let config = open_config(data_dir, &library_id)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| CoreError::Internal(format!("building tokio runtime: {e}")))?;

    let (db, stamper) = open_database(&config.library_dir, config.device_id.clone())?;

    let inventory = Inventory::new(db, stamper, config.library_dir);
    Ok(RunningApp { runtime, inventory })
}

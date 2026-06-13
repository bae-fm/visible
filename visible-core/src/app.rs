//! Composition root: open a library's database and hand back a running
//! application — the tokio runtime that owns the connection thread's async work
//! and the live [`Inventory`].

use coven::sync::session::SyncedTable;
use coven::Database;

use crate::error::CoreError;
use crate::library::open_config;
use crate::node::{Inventory, SCHEMA};

/// A fully opened library: the tokio runtime the bridge blocks on for database
/// calls, and the live node-tree service.
pub struct RunningApp {
    pub runtime: tokio::runtime::Runtime,
    pub inventory: Inventory,
}

/// Open `library_id` and bring up its [`Inventory`].
///
/// coven owns the connection: [`Database::open`] runs its bookkeeping migration,
/// then the `nodes` schema, seeds the `_updated_at` register off the rows on
/// disk, and hands back the non-optional stamper every node write binds. v1 is
/// local-only — no sync manager, no encryption, no cloud config. `nodes` is the
/// only host synced table; coven injects its own `item_keys`.
pub fn bootstrap(library_id: String) -> Result<RunningApp, CoreError> {
    let config = open_config(&library_id)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| CoreError::Internal(format!("building tokio runtime: {e}")))?;

    let (db, stamper) = Database::open(
        &config.library_dir.db_path(),
        vec![SyncedTable::new("nodes")],
        config.device_id.clone(),
        |conn| conn.execute_batch(SCHEMA).map_err(Into::into),
    )?;

    let inventory = Inventory::new(db, stamper, config.library_dir);
    Ok(RunningApp { runtime, inventory })
}

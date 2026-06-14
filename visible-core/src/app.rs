//! Composition root: open a library's database and hand back a running
//! application — the tokio runtime that owns the connection thread's async work,
//! the live [`Inventory`], and the [`Sync`] service.

use std::path::Path;
use std::sync::Arc;

use coven::clock::{ClockRef, SystemClock};
use coven::config::Config;
use coven::encryption::EncryptionService;
use coven::id_provider::{IdRef, UuidProvider};
use coven::keys::KeyService;
use coven::library_dir::LibraryDir;
use coven::sync::session::SyncedTable;
use coven::{Database, UpdatedAtStamper};
use tracing::warn;

use crate::error::CoreError;
use crate::library::open_config;
use crate::node::{Inventory, SCHEMA};
use crate::sync::Sync;

/// A fully opened library: the tokio runtime the bridge blocks on for database
/// and sync calls, the live node-tree service, and the cloud-sync service.
pub struct RunningApp {
    pub runtime: tokio::runtime::Runtime,
    pub inventory: Inventory,
    pub sync: Arc<Sync>,
}

/// The host synced tables every visible library carries: the node tree and the
/// immutable image-blob rows (`node_images` carries the image blobs, see
/// [`crate::blob_plan`]). The single source for both opening a library
/// ([`open_database`]) and joining/restoring one ([`crate::share`]), so the
/// schema contract can't drift between the two. coven injects its own `item_keys`.
pub(crate) fn synced_tables() -> Vec<SyncedTable> {
    vec![SyncedTable::new("nodes"), SyncedTable::new("node_images")]
}

/// Open the coven database for one library and run the schema. coven owns the
/// connection: [`Database::open`] runs its bookkeeping migration, then the schema
/// ([`SCHEMA`] creates `nodes` and `node_images`), seeds the `_updated_at`
/// register off the rows on disk, and hands back the non-optional stamper every
/// node write binds.
pub fn open_database(
    library_dir: &LibraryDir,
    device_id: String,
) -> Result<(Database, UpdatedAtStamper), CoreError> {
    Database::open(&library_dir.db_path(), synced_tables(), device_id, |conn| {
        conn.execute_batch(SCHEMA).map_err(Into::into)
    })
    .map_err(Into::into)
}

/// Run `work` on a thread with a 32 MB stack holding a multi-thread tokio runtime,
/// and hand that runtime to `work` by value. Opening the database, building the
/// sync manager, and `block_on`-ing the async sync setup (and the join/restore
/// snapshot download) nest deep async state machines that aren't collapsed in
/// debug builds, so they need more stack than the platform worker the bridge
/// calls from provides (a Swift Task or Android coroutine), which would overflow
/// and crash. The runtime's own workers get a 16 MB stack for the same reason.
///
/// `work` owns the runtime: [`bootstrap`] moves it into the returned [`RunningApp`]
/// (the bridge keeps blocking on it for later calls); the join/restore paths use
/// it for the one download and drop it when they return.
pub(crate) fn on_bootstrap_stack<T: Send + 'static>(
    name: &str,
    work: impl FnOnce(tokio::runtime::Runtime) -> Result<T, CoreError> + Send + 'static,
) -> Result<T, CoreError> {
    let name = name.to_string();
    let thread_name = name.clone();
    std::thread::Builder::new()
        .name(thread_name)
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .thread_stack_size(16 * 1024 * 1024)
                .enable_all()
                .build()
                .map_err(|e| CoreError::Internal(format!("building tokio runtime: {e}")))?;
            work(runtime)
        })
        .map_err(|e| CoreError::Internal(format!("spawning {name} thread: {e}")))?
        .join()
        .map_err(|_| CoreError::Internal(format!("{name} thread panicked")))?
}

/// Open `library_id` under `data_dir` and bring up its [`Inventory`] and
/// [`Sync`] service. Runs on the bootstrap stack (see [`on_bootstrap_stack`]),
/// whose runtime moves into the returned [`RunningApp`].
pub fn bootstrap(data_dir: &Path, library_id: String) -> Result<RunningApp, CoreError> {
    let data_dir = data_dir.to_path_buf();
    on_bootstrap_stack("visible-bootstrap", move |runtime| {
        bootstrap_inner(runtime, &data_dir, library_id)
    })
}

fn bootstrap_inner(
    runtime: tokio::runtime::Runtime,
    data_dir: &Path,
    library_id: String,
) -> Result<RunningApp, CoreError> {
    let config = open_config(data_dir, &library_id)?;

    let (db, stamper) = open_database(&config.library_dir, config.device_id.clone())?;

    // Composition root for the injected id source and wall clock — both passed
    // down to the data and sync layers.
    let ids: IdRef = Arc::new(UuidProvider);
    let clock: ClockRef = Arc::new(SystemClock);

    let inventory = Inventory::new(
        db.clone(),
        stamper,
        config.library_dir.clone(),
        ids,
        clock.clone(),
    );

    let key_service = KeyService::new(config.library_id.clone());

    // Resolve the encryption service only when this library already has a key on
    // this device — a returning user with a configured provider. A local-only
    // library has no key and needs none; the key is minted lazily, on first
    // connect (`Sync::save_s3_config`). The locked case — the key was set up but
    // the keyring lacks it on this device (OS keychain wiped, fresh install with
    // config preserved) — leaves sync unbuilt: minting a new key would orphan the
    // cloud data, so stay local until the user reconnects.
    let pending_enc = resolve_pending_encryption(&config, &key_service)?;

    let sync = Arc::new(Sync::new(
        config.clone(),
        key_service,
        db,
        clock,
        config.library_dir.clone(),
    ));

    // Resume sync at launch when the key is unlocked and a provider is
    // configured. A configured provider with no resolvable key (the locked case
    // above) stays local until the user reconnects.
    if let Some(enc) = pending_enc {
        if config.cloud_home.provider.is_some() {
            runtime.block_on(sync.attach_and_start(enc));
        }
    }

    Ok(RunningApp {
        runtime,
        inventory,
        sync,
    })
}

/// Resolve the encryption service for a returning user, or `None` to stay local.
///
/// `None` when no key was ever stored (local-only library) or when the key is
/// marked stored but absent from this device's keyring (the locked case — warned
/// and left for the user to reconnect). `Err` only when a present key is
/// malformed (corruption, surfaced rather than masked).
fn resolve_pending_encryption(
    config: &Config,
    key_service: &KeyService,
) -> Result<Option<EncryptionService>, CoreError> {
    if !config.encryption_key_stored {
        return Ok(None);
    }
    match key_service.get_encryption_key()? {
        Some(key_hex) => Ok(Some(EncryptionService::new(&key_hex)?)),
        None => {
            warn!(
                "encryption key marked stored but not found in keyring; \
                 deferring sync until the provider is reconnected"
            );
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    fn config(stored: bool) -> (Config, TempDir) {
        let temp = TempDir::new().unwrap();
        let dir = LibraryDir::new(temp.path().join("library"));
        let mut config = Config::with_defaults(
            "lib-locked".to_string(),
            "device".to_string(),
            dir,
            "Home".to_string(),
        );
        config.encryption_key_stored = stored;
        (config, temp)
    }

    #[test]
    fn no_stored_key_resolves_to_local_only() {
        crate::config::install_test_keyring();
        let (config, _temp) = config(false);
        let key_service = KeyService::new("lib-locked-absent".to_string());
        // A local-only library never recorded a key; sync stays unbuilt.
        let resolved = resolve_pending_encryption(&config, &key_service).unwrap();
        assert!(resolved.is_none());
    }

    #[test]
    fn key_marked_stored_but_absent_stays_local_without_crashing() {
        crate::config::install_test_keyring();
        // The config says a key was set up, but this device's keyring lacks it
        // (OS keychain wiped, fresh install with config preserved). Minting a new
        // key would orphan the cloud data, so resolution must defer to local —
        // returning None and warning, never panicking or minting.
        let (config, _temp) = config(true);
        let key_service = KeyService::new("lib-locked-missing".to_string());
        let resolved = resolve_pending_encryption(&config, &key_service).unwrap();
        assert!(resolved.is_none());
    }
}

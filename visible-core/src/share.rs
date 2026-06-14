//! Joining and leaving shared libraries: the host-level operations a joiner runs
//! before it has an [`crate::app::RunningApp`] for the target library.
//!
//! coven owns the join/restore protocol (decode the code, build the cloud home,
//! download the snapshot, write the library + config); these functions wire it to
//! visible's synced tables and blob plan, then hand back the joined library's
//! identity for the host to open. [`remove_library`] is the other half of the
//! single-active-home model: after a successful join the host opens the new
//! library and drops the prior one's on-disk directory.

use std::path::Path;
use std::sync::Arc;

use coven::blob::BlobPlan;
use coven::clock::{ClockRef, SystemClock};
use coven::id_provider::{IdRef, UuidProvider};
use coven::keys::KeyService;
use coven::library_dir::LibraryDir;
use coven::sync::join::join_from_invite_code;
use coven::sync::restore::restore_from_code;
use coven::sync::session::SyncedTable;
use tracing::{debug, warn};

use crate::blob_plan::NodeBlobPlan;
use crate::error::CoreError;
use crate::library::LibraryInfo;

/// The synced tables every visible library carries: the node tree and the image
/// blob rows. The single source for the join/restore set, matching what
/// [`crate::app::open_database`] opens.
fn synced_tables() -> [SyncedTable; 2] {
    [SyncedTable::new("nodes"), SyncedTable::new("node_images")]
}

/// Build the blob-plan factory the join/restore protocol calls once it has
/// created the library directory: visible's [`NodeBlobPlan`] bound to that dir.
fn make_blob_plan(dir: &LibraryDir) -> Box<dyn BlobPlan> {
    Box::new(NodeBlobPlan::new(dir.clone()))
}

/// Join a shared library from an invite code: decode it, build the cloud home,
/// download the snapshot, and write the new library + its config under
/// `data_dir`. Returns the joined library's identity so the host can open it.
///
/// S3 needs no OAuth, so the OAuth cancel channel is created already false and
/// never tripped; CloudKit isn't a visible provider, so no CloudKit driver is
/// passed. Runs on the bootstrap stack (the snapshot download nests deep async
/// state machines that overflow the platform worker thread).
pub fn join_shared_library(data_dir: &Path, invite_code: &str) -> Result<LibraryInfo, CoreError> {
    run_on_bootstrap_stack("visible-join", {
        let data_dir = data_dir.to_path_buf();
        let invite_code = invite_code.to_string();
        move |runtime| {
            // Composition root for the injected id source and wall clock, as in
            // `app::bootstrap` — the joined library's first device id and the
            // clock its snapshot pull stamps against.
            let ids: IdRef = Arc::new(UuidProvider);
            let clock: ClockRef = Arc::new(SystemClock);

            // S3 needs no interactive OAuth, so this receiver is never set true;
            // coven's S3 join path doesn't read it. The sender is dropped at the
            // end of the closure, after the join has run.
            let (_oauth_cancel_tx, oauth_cancel_rx) = tokio::sync::watch::channel(false);
            let config = runtime
                .block_on(join_from_invite_code(
                    &invite_code,
                    &data_dir,
                    &synced_tables(),
                    None,
                    oauth_cancel_rx,
                    clock,
                    ids,
                    make_blob_plan,
                    |status| debug!(status, "joining shared library"),
                ))
                .map_err(|e| CoreError::Sync(e.to_string()))?;
            Ok(LibraryInfo::from(&config))
        }
    })
}

/// Restore a library from an owner's restore code: decode it, rebuild the cloud
/// home, download the snapshot, and write the library + config under `data_dir`.
/// Returns the restored library's identity for the host to open.
///
/// Restore carries the encryption and signing keys in the code itself, so it
/// needs no OAuth tokens or CloudKit driver for visible's S3 libraries. Runs on
/// the bootstrap stack for the same deep-async reason as [`join_shared_library`].
pub fn restore_shared_library(
    data_dir: &Path,
    restore_code: &str,
) -> Result<LibraryInfo, CoreError> {
    run_on_bootstrap_stack("visible-restore", {
        let data_dir = data_dir.to_path_buf();
        let restore_code = restore_code.to_string();
        move |runtime| {
            let ids: IdRef = Arc::new(UuidProvider);
            let clock: ClockRef = Arc::new(SystemClock);
            let config = runtime
                .block_on(restore_from_code(
                    &restore_code,
                    &synced_tables(),
                    None,
                    None,
                    &data_dir,
                    clock,
                    ids,
                    make_blob_plan,
                    |status| debug!(status, "restoring library"),
                ))
                .map_err(|e| CoreError::Sync(e.to_string()))?;
            Ok(LibraryInfo::from(&config))
        }
    })
}

/// Remove a library from this device: delete its on-disk directory tree and clear
/// its keyring entries (the per-library encryption key and cloud credentials).
/// The other half of the single-active-home model — after a successful join the
/// host drops the prior library.
///
/// The directory removal is the operation; the keyring clears are best-effort
/// (the directory is already gone, so a lingering inert keyring entry can't bring
/// the library back). The global identity keypair is shared across all libraries
/// and is never deleted here.
pub fn remove_library(data_dir: &Path, library_id: &str) -> Result<(), CoreError> {
    let dir = LibraryDir::new(data_dir.join("libraries").join(library_id));
    std::fs::remove_dir_all(&*dir)
        .map_err(|e| CoreError::Io(format!("removing library directory {}: {e}", dir.display())))?;

    let key_service = KeyService::new(library_id.to_string());
    if let Err(e) = key_service.delete_encryption_key() {
        warn!(
            library_id,
            "failed to delete encryption key on remove_library: {e}"
        );
    }
    if let Err(e) = key_service.delete_cloud_home_credentials() {
        warn!(
            library_id,
            "failed to delete cloud credentials on remove_library: {e}"
        );
    }
    Ok(())
}

/// Run `work` on a thread with a 32 MB stack holding a multi-thread tokio runtime
/// — the same stack [`crate::app::bootstrap`] uses, for the same reason: the
/// snapshot download nests deep async state machines that aren't collapsed in
/// debug builds and overflow the platform worker thread the bridge calls from.
fn run_on_bootstrap_stack<T: Send + 'static>(
    name: &str,
    work: impl FnOnce(&tokio::runtime::Runtime) -> Result<T, CoreError> + Send + 'static,
) -> Result<T, CoreError> {
    std::thread::Builder::new()
        .name(name.to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .thread_stack_size(16 * 1024 * 1024)
                .enable_all()
                .build()
                .map_err(|e| CoreError::Internal(format!("building tokio runtime: {e}")))?;
            work(&runtime)
        })
        .map_err(|e| CoreError::Internal(format!("spawning {name} thread: {e}")))?
        .join()
        .map_err(|_| CoreError::Internal(format!("{name} thread panicked")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    use coven::id_provider::UuidProvider;

    /// remove_library deletes the library's on-disk directory tree. Create a
    /// throwaway library through the real create path, then assert remove_library
    /// drops its directory.
    #[test]
    fn remove_library_deletes_the_on_disk_directory() {
        crate::config::install_test_keyring();

        let temp = tempfile::TempDir::new().unwrap();
        let info =
            crate::library::create(temp.path(), "Shared".to_string(), &UuidProvider).unwrap();

        let dir = LibraryDir::new(temp.path().join("libraries").join(&info.id));
        assert!(dir.exists(), "library dir exists after create");

        remove_library(temp.path(), &info.id).unwrap();
        assert!(!dir.exists(), "library dir is gone after remove_library");
    }
}

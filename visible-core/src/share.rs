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
use tracing::{debug, warn};

use crate::app::{on_bootstrap_stack, synced_tables};
use crate::blob_plan::NodeBlobPlan;
use crate::error::CoreError;
use crate::library::{library_dir, LibraryInfo};

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
/// passed. Runs on the bootstrap stack (see [`on_bootstrap_stack`]); the runtime
/// the snapshot download blocks on is dropped when the join returns.
pub fn join_shared_library(data_dir: &Path, invite_code: &str) -> Result<LibraryInfo, CoreError> {
    let data_dir = data_dir.to_path_buf();
    let invite_code = invite_code.to_string();
    on_bootstrap_stack("visible-join", move |runtime| {
        // Composition root for the injected id source and wall clock, as in
        // `app::bootstrap` — the joined library's first device id and the clock
        // its snapshot pull stamps against.
        let ids: IdRef = Arc::new(UuidProvider);
        let clock: ClockRef = Arc::new(SystemClock);

        // S3 needs no interactive OAuth, so this receiver is never set true;
        // coven's S3 join path doesn't read it. The sender is dropped at the end
        // of the closure, after the join has run.
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
    let data_dir = data_dir.to_path_buf();
    let restore_code = restore_code.to_string();
    on_bootstrap_stack("visible-restore", move |runtime| {
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
    let dir = library_dir(data_dir, library_id);
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

/// This device's identity code: its global Ed25519 public key as hex, read (and
/// minted if absent) from the keyring. The signing keypair is global — its
/// keyring account is fixed, not scoped to any library — so the `library_id`
/// passed to [`KeyService`] is immaterial here, and no `data_dir` is needed: the
/// code is available before any library exists. The onboarding "join a home" flow
/// shows it so a joiner can send it to a home's owner before they have a library
/// to open. ([`Sync::user_identity_code`](crate::Sync::user_identity_code) reads
/// the same keypair once a library is open.)
pub fn user_identity_code() -> Result<String, CoreError> {
    let key_service = KeyService::new(String::new());
    let keypair = key_service.get_or_create_user_keypair()?;
    Ok(hex::encode(keypair.public_key))
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

        let dir = library_dir(temp.path(), &info.id);
        assert!(dir.exists(), "library dir exists after create");

        remove_library(temp.path(), &info.id).unwrap();
        assert!(!dir.exists(), "library dir is gone after remove_library");
    }

    /// The pre-library identity code is the global keypair as hex, minted on
    /// demand and stable across calls — available with no library on disk, which
    /// is what the onboarding "join a home" screen needs.
    #[test]
    fn user_identity_code_is_a_stable_hex_pubkey_with_no_library() {
        crate::config::install_test_keyring();

        let code = user_identity_code().unwrap();
        assert_eq!(code.len(), 64, "32-byte pubkey as hex");
        assert!(
            code.bytes().all(|b| b.is_ascii_hexdigit()),
            "identity code is hex: {code}"
        );
        // Read back, not re-minted.
        assert_eq!(user_identity_code().unwrap(), code);
    }
}

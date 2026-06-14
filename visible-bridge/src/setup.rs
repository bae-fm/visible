//! Pre-AppHandle free functions: creating or discovering libraries under the
//! host-supplied data directory, before any library is open. The host passes its
//! private files directory (e.g. Android `Context.filesDir`, the iOS app
//! support directory) as `data_dir`.

use std::path::PathBuf;

use crate::types::{BridgeError, BridgeLibrary};

/// Create a new library named `name` under `data_dir` and return it. The
/// onboarding "create a home" flow names the home; the name becomes both the
/// library name and the root house node's title.
#[uniffi::export]
pub fn create_library(data_dir: String, name: String) -> Result<BridgeLibrary, BridgeError> {
    Ok(visible_core::library::create_named(&PathBuf::from(data_dir), name)?.into())
}

/// This device's identity code (its Ed25519 public key, hex), readable before any
/// library exists. The onboarding "join a home" flow shows it so a joiner can send
/// it to a home's owner out of band before they have a library to open. Once a
/// library is open, `AppHandle::user_identity_code` returns the same code.
#[uniffi::export]
pub fn user_identity_code() -> Result<String, BridgeError> {
    Ok(visible_core::user_identity_code()?)
}

/// Every library found under `data_dir`.
#[uniffi::export]
pub fn discover_libraries(data_dir: String) -> Result<Vec<BridgeLibrary>, BridgeError> {
    Ok(visible_core::library::discover(&PathBuf::from(data_dir))?
        .into_iter()
        .map(BridgeLibrary::from)
        .collect())
}

/// Join a shared library from an invite code: download its snapshot and write the
/// library under `data_dir`, returning its identity. The app then opens it with
/// `init_app(data_dir, joined_id)` and drops the prior library with
/// `remove_library(data_dir, old_id)` (single active home).
#[uniffi::export]
pub fn join_library_from_invite(
    data_dir: String,
    invite_code: String,
) -> Result<BridgeLibrary, BridgeError> {
    Ok(visible_core::join_shared_library(&PathBuf::from(data_dir), &invite_code)?.into())
}

/// Restore a library from an owner's restore code: download its snapshot and write
/// the library under `data_dir`, returning its identity for the app to open.
#[uniffi::export]
pub fn restore_library_from_code(
    data_dir: String,
    restore_code: String,
) -> Result<BridgeLibrary, BridgeError> {
    Ok(visible_core::restore_shared_library(&PathBuf::from(data_dir), &restore_code)?.into())
}

/// Remove a library from this device: delete its on-disk directory and clear its
/// keyring entries. The app calls this to drop the prior library after a join
/// (single active home).
#[uniffi::export]
pub fn remove_library(data_dir: String, library_id: String) -> Result<(), BridgeError> {
    Ok(visible_core::remove_library(
        &PathBuf::from(data_dir),
        &library_id,
    )?)
}

/// Install the platform keyring store and name coven's keyring service. Apps call
/// this once at startup, before opening a session — cloud sync reads the identity
/// keypair and the per-library encryption key from the keyring.
#[uniffi::export]
pub fn init_keyring() {
    visible_core::init_keyring();
}

/// Point the TLS stack at the OS certificate-authority store via `SSL_CERT_DIR`
/// (a colon-separated list of directories honored by `rustls-native-certs`, which
/// the S3 client uses). Android exposes its trusted roots as PEM files under
/// these directories but not on the POSIX paths the cert loader probes by
/// default, so without this every S3 TLS handshake fails. The Android caller
/// passes its system cert directories. Called once at startup before any worker
/// thread reads the environment, so the set is race-free.
#[cfg(target_os = "android")]
#[uniffi::export]
pub fn set_ca_cert_dir(dirs: String) {
    std::env::set_var("SSL_CERT_DIR", dirs);
}

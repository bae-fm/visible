//! Pre-AppHandle free functions: pointing the data dir at the host's storage,
//! and creating or discovering libraries — all before any library is open.

use crate::types::{BridgeError, BridgeLibrary};

/// Point visible's data directory at `path/.visible` by exporting `path` as
/// `$HOME`, which is what `dirs::home_dir()` (and thus `data_dir()` / library
/// discovery / `init_app`) resolves against. Mobile app processes don't get a
/// `$HOME`, so the native app MUST call this once at startup — before
/// `discover_libraries`, `create_library`, or `init_app` — passing its private
/// files directory (e.g. Android `Context.filesDir`). Without it those calls
/// fail with "could not determine home directory".
#[cfg(any(target_os = "ios", target_os = "android"))]
#[uniffi::export]
pub fn set_data_dir(path: String) {
    // Called once at process startup before any worker thread reads the
    // environment, so the set is race-free.
    std::env::set_var("HOME", path);
}

/// Create the default first library ("Home") and return it.
#[uniffi::export]
pub fn create_library() -> Result<BridgeLibrary, BridgeError> {
    let info = visible_core::library::create_default(&coven::id_provider::UuidProvider)?;
    Ok(BridgeLibrary {
        id: info.id,
        name: info.name,
    })
}

/// Every library found under the data directory.
#[uniffi::export]
pub fn discover_libraries() -> Result<Vec<BridgeLibrary>, BridgeError> {
    Ok(visible_core::library::discover()
        .into_iter()
        .map(|info| BridgeLibrary {
            id: info.id,
            name: info.name,
        })
        .collect())
}

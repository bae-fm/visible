//! Pre-AppHandle free functions: creating or discovering libraries under the
//! host-supplied data directory, before any library is open. The host passes its
//! private files directory (e.g. Android `Context.filesDir`, the iOS app
//! support directory) as `data_dir`.

use std::path::PathBuf;

use crate::types::{BridgeError, BridgeLibrary};

/// Create the default first library ("Home") under `data_dir` and return it.
#[uniffi::export]
pub fn create_library(data_dir: String) -> Result<BridgeLibrary, BridgeError> {
    Ok(visible_core::library::create_default(&PathBuf::from(data_dir))?.into())
}

/// Every library found under `data_dir`.
#[uniffi::export]
pub fn discover_libraries(data_dir: String) -> Result<Vec<BridgeLibrary>, BridgeError> {
    Ok(visible_core::library::discover(&PathBuf::from(data_dir))?
        .into_iter()
        .map(BridgeLibrary::from)
        .collect())
}

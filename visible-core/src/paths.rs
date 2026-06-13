//! The on-disk root for visible's data.

use std::path::PathBuf;

use crate::error::CoreError;

/// The directory visible stores everything under: `~/.visible`. Libraries live
/// in `data_dir()/libraries/`. Errors if the home directory can't be resolved —
/// on mobile the host points `$HOME` at its private files directory before any
/// core call, so an unresolvable home is a wiring bug, not a default to paper
/// over.
pub fn data_dir() -> Result<PathBuf, CoreError> {
    let home = dirs::home_dir()
        .ok_or_else(|| CoreError::Io("could not determine home directory".into()))?;
    Ok(home.join(".visible"))
}

//! Library lifecycle: discovery, creation, and config loading. No live database
//! session — that is [`crate::app::bootstrap`]'s job. Creation opens the DB once
//! transiently to lay down the root node, then drops the handle.

use std::path::Path;

use coven::config::{Config, ConfigYaml};
use coven::id_provider::IdProvider;
use coven::library_dir::LibraryDir;
use tracing::{debug, warn};

use crate::app::open_database;
use crate::error::CoreError;
use crate::node;

/// A library as seen from outside an open session: its identity, for the
/// picker. The live tree comes from [`crate::app::bootstrap`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryInfo {
    pub id: String,
    pub name: String,
}

impl LibraryInfo {
    fn from_config_yaml(yaml: &ConfigYaml) -> Self {
        Self {
            id: yaml.library_id.clone(),
            name: yaml.library_name.clone(),
        }
    }
}

/// Every library found under `data_dir/libraries/`. A directory whose
/// `config.yaml` is missing or unparseable is skipped with a log — a half
/// written or foreign directory shouldn't sink the whole list. A missing
/// `libraries/` directory is the normal first-launch state (empty list); any
/// other failure to read it surfaces as an error.
pub fn discover(data_dir: &Path) -> Result<Vec<LibraryInfo>, CoreError> {
    let libraries_dir = data_dir.join("libraries");
    let entries = match std::fs::read_dir(&libraries_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!(dir = %libraries_dir.display(), "no libraries directory yet");
            return Ok(Vec::new());
        }
        Err(e) => {
            return Err(CoreError::Io(format!(
                "reading libraries dir {}: {e}",
                libraries_dir.display()
            )));
        }
    };

    let mut out = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                warn!(dir = %libraries_dir.display(), "skipping unreadable library dir entry: {e}");
                continue;
            }
        };
        let config_path = LibraryDir::new(entry.path()).config_path();
        let text = match std::fs::read_to_string(&config_path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!(path = %config_path.display(), "skipping non-library entry: no config");
                continue;
            }
            Err(e) => {
                warn!(path = %config_path.display(), "skipping library, cannot read config: {e}");
                continue;
            }
        };
        match serde_yaml::from_str::<ConfigYaml>(&text) {
            Ok(yaml) => out.push(LibraryInfo::from_config_yaml(&yaml)),
            Err(e) => {
                warn!(path = %config_path.display(), "skipping library, config failed to parse: {e}")
            }
        }
    }
    Ok(out)
}

/// Create a new library named `name` under `data_dir`: a fresh id, the on-disk
/// directory + `config.yaml` (with a generated device id), and the root house
/// node.
///
/// The database is opened once here to write the root, then the handle drops at
/// the end of the call. The live session is opened later by
/// [`crate::app::bootstrap`].
pub fn create(
    data_dir: &Path,
    name: String,
    ids: &dyn IdProvider,
) -> Result<LibraryInfo, CoreError> {
    let library_id = ids.new_id();
    let config = LibraryDir::create(data_dir, library_id, name.clone(), ids)?;

    // Open the DB once to lay down coven's bookkeeping schema, the nodes schema,
    // and the root house node, then let the handle drop.
    let (db, stamper) = open_database(&config.library_dir, config.device_id.clone())?;

    let root_id = ids.new_id();
    let updated_at = stamper.stamp();
    let root_name = name.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CoreError::Internal(format!("building runtime for library create: {e}")))?;
    runtime.block_on(db.call(move |conn| {
        node::insert_root(conn, &root_id, &root_name, &updated_at).map_err(Into::into)
    }))?;

    Ok(LibraryInfo {
        id: config.library_id,
        name,
    })
}

/// Create the default first library ("Home") under `data_dir`.
pub fn create_default(data_dir: &Path) -> Result<LibraryInfo, CoreError> {
    create(
        data_dir,
        "Home".to_string(),
        &coven::id_provider::UuidProvider,
    )
}

/// Load the runtime [`Config`] for `library_id` under `data_dir` from its
/// `config.yaml`. The device id comes from the yaml; its absence is corruption
/// (greenfield always writes it), surfaced as an error rather than defaulted.
pub fn open_config(data_dir: &Path, library_id: &str) -> Result<Config, CoreError> {
    let library_dir = LibraryDir::new(data_dir.join("libraries").join(library_id));
    let text = std::fs::read_to_string(library_dir.config_path())
        .map_err(|e| CoreError::Config(format!("reading config for library {library_id}: {e}")))?;
    let yaml: ConfigYaml = serde_yaml::from_str(&text)
        .map_err(|e| CoreError::Config(format!("parsing config for library {library_id}: {e}")))?;
    let device_id = yaml.device_id.clone().ok_or_else(|| {
        CoreError::Config(format!("library {library_id} config has no device_id"))
    })?;
    Ok(yaml.into_config(device_id, library_dir))
}

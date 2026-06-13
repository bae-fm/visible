//! Library lifecycle: discovery, creation, and config loading. No live database
//! session — that is [`crate::app::bootstrap`]'s job. Creation opens the DB once
//! transiently to lay down the root node, then drops the handle.

use coven::config::{Config, ConfigYaml};
use coven::id_provider::IdProvider;
use coven::library_dir::LibraryDir;
use coven::sync::session::SyncedTable;
use coven::Database;
use tracing::warn;

use crate::error::CoreError;
use crate::node::{self, SCHEMA};
use crate::paths::data_dir;

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

/// Every library found under `data_dir()/libraries/`. A directory whose
/// `config.yaml` is missing or unparseable is skipped with a `warn!` — a half
/// written or foreign directory shouldn't sink the whole list.
pub fn discover() -> Vec<LibraryInfo> {
    let libraries_dir = match data_dir() {
        Ok(d) => d.join("libraries"),
        Err(e) => {
            warn!("cannot resolve data dir for library discovery: {e}");
            return Vec::new();
        }
    };
    let entries = match std::fs::read_dir(&libraries_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            warn!(dir = %libraries_dir.display(), "cannot read libraries dir: {e}");
            return Vec::new();
        }
    };

    let mut out = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                warn!("skipping unreadable library dir entry: {e}");
                continue;
            }
        };
        let config_path = LibraryDir::new(entry.path()).config_path();
        let text = match std::fs::read_to_string(&config_path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
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
    out
}

/// Create a new library named `name`: a fresh id, the on-disk directory +
/// `config.yaml` (with a generated device id), and the root house node.
///
/// The database is opened once here to write the root, then the handle drops at
/// the end of the call. The live session is opened later by
/// [`crate::app::bootstrap`].
pub fn create(name: String, ids: &dyn IdProvider) -> Result<LibraryInfo, CoreError> {
    let data_dir = data_dir()?;
    let library_id = ids.new_id();
    let config = LibraryDir::create(&data_dir, library_id, name.clone(), ids)?;

    // Open the DB once to lay down coven's bookkeeping schema, the nodes schema,
    // and the root house node, then let the handle drop.
    let (db, stamper) = Database::open(
        &config.library_dir.db_path(),
        vec![SyncedTable::new("nodes")],
        config.device_id.clone(),
        |conn| conn.execute_batch(SCHEMA).map_err(Into::into),
    )?;

    let root_id = ids.new_id();
    let created_at = chrono::Utc::now().to_rfc3339();
    let updated_at = stamper.stamp();
    let root_name = name.clone();
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| CoreError::Internal(format!("building runtime for library create: {e}")))?;
    runtime.block_on(db.call(move |conn| {
        node::insert_root(conn, &root_id, &root_name, &created_at, &updated_at).map_err(Into::into)
    }))?;

    Ok(LibraryInfo {
        id: config.library_id,
        name,
    })
}

/// Create the default first library ("Home").
pub fn create_default(ids: &dyn IdProvider) -> Result<LibraryInfo, CoreError> {
    create("Home".to_string(), ids)
}

/// Load the runtime [`Config`] for `library_id` from its `config.yaml`. The
/// device id comes from the yaml; its absence is corruption (greenfield always
/// writes it), surfaced as an error rather than defaulted.
pub fn open_config(library_id: &str) -> Result<Config, CoreError> {
    let library_dir = LibraryDir::new(data_dir()?.join("libraries").join(library_id));
    let text = std::fs::read_to_string(library_dir.config_path())
        .map_err(|e| CoreError::Config(format!("reading config for library {library_id}: {e}")))?;
    let yaml: ConfigYaml = serde_yaml::from_str(&text)
        .map_err(|e| CoreError::Config(format!("parsing config for library {library_id}: {e}")))?;
    let device_id = yaml.device_id.clone().ok_or_else(|| {
        CoreError::Config(format!("library {library_id} config has no device_id"))
    })?;
    Ok(yaml.into_config(device_id, library_dir))
}

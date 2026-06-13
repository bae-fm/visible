//! Tests the real library lifecycle (`create` / `discover` / `open_config`)
//! against a temp data dir, plus `bootstrap` opening the created library and
//! finding its root.

use coven::id_provider::SequentialIdProvider;
use tempfile::TempDir;
use visible_core::app::bootstrap;
use visible_core::library::{create, create_default, discover, open_config};

#[test]
fn create_then_discover_finds_the_library() {
    let data_dir = TempDir::new().expect("temp data dir");
    let ids = SequentialIdProvider::new("lib");

    let info = create(data_dir.path(), "Garage Sale".to_string(), &ids).expect("create");
    assert_eq!(info.name, "Garage Sale");

    let found = discover(data_dir.path()).expect("discover");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, info.id);
    assert_eq!(found[0].name, "Garage Sale");
}

#[test]
fn discover_on_missing_data_dir_is_empty() {
    let data_dir = TempDir::new().expect("temp data dir");
    let found = discover(data_dir.path()).expect("discover");
    assert!(found.is_empty());
}

#[test]
fn create_default_names_the_library_home() {
    let data_dir = TempDir::new().expect("temp data dir");

    let info = create_default(data_dir.path()).expect("create default");
    assert_eq!(info.name, "Home");
}

#[test]
fn open_config_reads_back_the_device_id() {
    let data_dir = TempDir::new().expect("temp data dir");
    let ids = SequentialIdProvider::new("lib");

    let info = create(data_dir.path(), "Place".to_string(), &ids).expect("create");
    let config = open_config(data_dir.path(), &info.id).expect("open config");
    assert_eq!(config.library_id, info.id);
    assert_eq!(config.library_name, "Place");
    assert!(!config.device_id.is_empty());
}

#[test]
fn bootstrap_opens_the_created_library_with_its_root() {
    let data_dir = TempDir::new().expect("temp data dir");
    let ids = SequentialIdProvider::new("lib");

    let info = create(data_dir.path(), "Place".to_string(), &ids).expect("create");
    let app = bootstrap(data_dir.path(), info.id).expect("bootstrap");

    let root = app
        .runtime
        .block_on(app.inventory.root())
        .expect("root after bootstrap");
    assert_eq!(root.name, "Place");
    assert_eq!(root.parent_id, None);

    // The freshly created library has no children under the root yet.
    let children = app
        .runtime
        .block_on(app.inventory.children(&root.id))
        .expect("children");
    assert!(children.is_empty());
}

//! Tests the real library lifecycle (`create` / `discover` / `open_config`)
//! against a redirected `$HOME`, plus `bootstrap` opening the created library
//! and finding its root. `$HOME` is process-global, so these run serially.

use coven::id_provider::SequentialIdProvider;
use serial_test::serial;
use tempfile::TempDir;
use visible_core::app::bootstrap;
use visible_core::library::{create, create_default, discover, open_config};

/// Point `$HOME` at a fresh temp dir for the duration of one test, so
/// `data_dir()` resolves under it. Returns the guard dir, which must outlive the
/// test body.
fn with_temp_home() -> TempDir {
    let temp = TempDir::new().expect("temp home");
    std::env::set_var("HOME", temp.path());
    temp
}

#[test]
#[serial]
fn create_then_discover_finds_the_library() {
    let _home = with_temp_home();
    let ids = SequentialIdProvider::new("lib");

    let info = create("Garage Sale".to_string(), &ids).expect("create");
    assert_eq!(info.name, "Garage Sale");

    let found = discover();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, info.id);
    assert_eq!(found[0].name, "Garage Sale");
}

#[test]
#[serial]
fn create_default_names_the_library_home() {
    let _home = with_temp_home();
    let ids = SequentialIdProvider::new("lib");

    let info = create_default(&ids).expect("create default");
    assert_eq!(info.name, "Home");
}

#[test]
#[serial]
fn open_config_reads_back_the_device_id() {
    let _home = with_temp_home();
    let ids = SequentialIdProvider::new("lib");

    let info = create("Place".to_string(), &ids).expect("create");
    let config = open_config(&info.id).expect("open config");
    assert_eq!(config.library_id, info.id);
    assert_eq!(config.library_name, "Place");
    assert!(!config.device_id.is_empty());
}

#[test]
#[serial]
fn bootstrap_opens_the_created_library_with_its_root() {
    let _home = with_temp_home();
    let ids = SequentialIdProvider::new("lib");

    let info = create("Place".to_string(), &ids).expect("create");
    let app = bootstrap(info.id).expect("bootstrap");

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

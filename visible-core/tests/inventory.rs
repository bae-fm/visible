//! Tests the real [`Inventory`] against a real coven [`Database`] on a temp
//! directory — the production unit, not a reconstruction.

use coven::library_dir::LibraryDir;
use coven::{Database, UpdatedAtStamper};
use tempfile::TempDir;
use visible_core::app::open_database;
use visible_core::node::insert_root;
use visible_core::Inventory;

/// Open a real database on a fresh temp library dir, lay down the schema and a
/// root house node, and return the live inventory (plus the tempdir, which must
/// outlive it). Mirrors what `library::create` + `app::bootstrap` do, so the
/// test drives the production node-tree code.
async fn open_inventory() -> (Inventory, TempDir) {
    let temp = TempDir::new().expect("temp dir");
    let dir = LibraryDir::new(temp.path().join("library"));
    std::fs::create_dir_all(&*dir).expect("create library dir");

    let (db, stamper) = open_database(&dir, "test-device".to_string()).expect("open database");

    seed_root(&db, &stamper).await;

    (Inventory::new(db, stamper, dir), temp)
}

async fn seed_root(db: &Database, stamper: &UpdatedAtStamper) {
    let updated_at = stamper.stamp();
    db.call(move |conn| insert_root(conn, "root", "Home", &updated_at).map_err(Into::into))
        .await
        .expect("seed root");
}

#[tokio::test]
async fn root_exists_after_create() {
    let (inv, _temp) = open_inventory().await;
    let root = inv.root().await.expect("root");
    assert_eq!(root.id, "root");
    assert_eq!(root.name, "Home");
    assert_eq!(root.parent_id, None);
    assert_eq!(root.position, 0);
}

#[tokio::test]
async fn children_keep_insertion_order() {
    let (inv, _temp) = open_inventory().await;

    let kitchen = inv.create_child("root", "Kitchen".into()).await.unwrap();
    let bedroom = inv.create_child("root", "Bedroom".into()).await.unwrap();
    let garage = inv.create_child("root", "Garage".into()).await.unwrap();

    assert_eq!(kitchen.position, 0);
    assert_eq!(bedroom.position, 1);
    assert_eq!(garage.position, 2);

    let children = inv.children("root").await.unwrap();
    let names: Vec<&str> = children.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, vec!["Kitchen", "Bedroom", "Garage"]);
}

#[tokio::test]
async fn rename_changes_the_name() {
    let (inv, _temp) = open_inventory().await;
    let node = inv.create_child("root", "Shelf".into()).await.unwrap();

    inv.rename(&node.id, "Bookshelf".into()).await.unwrap();

    let reloaded = inv.get(&node.id).await.unwrap().unwrap();
    assert_eq!(reloaded.name, "Bookshelf");
}

#[tokio::test]
async fn rename_missing_node_is_not_found() {
    let (inv, _temp) = open_inventory().await;
    let err = inv.rename("nope", "X".into()).await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn delete_removes_subtree_and_its_image_files() {
    let (inv, _temp) = open_inventory().await;

    // root -> room -> box (with an image) -> thing (with an image)
    let room = inv.create_child("root", "Room".into()).await.unwrap();
    let container = inv.create_child(&room.id, "Box".into()).await.unwrap();
    let thing = inv
        .create_child(&container.id, "Thing".into())
        .await
        .unwrap();

    inv.set_image(&container.id, b"box-photo".to_vec())
        .await
        .unwrap();
    inv.set_image(&thing.id, b"thing-photo".to_vec())
        .await
        .unwrap();

    let box_image = inv
        .get(&container.id)
        .await
        .unwrap()
        .unwrap()
        .image_id
        .unwrap();
    let thing_image = inv.get(&thing.id).await.unwrap().unwrap().image_id.unwrap();
    assert!(inv.image_path_if_exists(&box_image).is_some());
    assert!(inv.image_path_if_exists(&thing_image).is_some());

    inv.delete(&room.id).await.unwrap();

    // The whole subtree is gone.
    assert!(inv.get(&room.id).await.unwrap().is_none());
    assert!(inv.get(&container.id).await.unwrap().is_none());
    assert!(inv.get(&thing.id).await.unwrap().is_none());

    // And so are their image files.
    assert!(inv.image_path_if_exists(&box_image).is_none());
    assert!(inv.image_path_if_exists(&thing_image).is_none());

    // The root is untouched.
    assert!(inv.root().await.is_ok());
}

#[tokio::test]
async fn delete_missing_node_is_not_found() {
    let (inv, _temp) = open_inventory().await;
    let err = inv.delete("nope").await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn delete_root_errors_and_leaves_the_tree_intact() {
    let (inv, _temp) = open_inventory().await;
    let room = inv.create_child("root", "Room".into()).await.unwrap();

    let err = inv.delete("root").await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::Internal(_)),
        "{err:?}"
    );

    // The root and its subtree are untouched; a non-root delete still works.
    assert!(inv.root().await.is_ok());
    assert!(inv.get(&room.id).await.unwrap().is_some());
    inv.delete(&room.id).await.unwrap();
    assert!(inv.get(&room.id).await.unwrap().is_none());
    assert!(inv.root().await.is_ok());
}

#[tokio::test]
async fn set_image_writes_then_replacing_removes_the_old_file() {
    let (inv, _temp) = open_inventory().await;
    let node = inv.create_child("root", "Lamp".into()).await.unwrap();

    inv.set_image(&node.id, b"first".to_vec()).await.unwrap();
    let first_image = inv.get(&node.id).await.unwrap().unwrap().image_id.unwrap();
    let first_path = inv
        .image_path_if_exists(&first_image)
        .expect("first image on disk");
    assert_eq!(std::fs::read(&first_path).unwrap(), b"first");

    inv.set_image(&node.id, b"second".to_vec()).await.unwrap();
    let second_image = inv.get(&node.id).await.unwrap().unwrap().image_id.unwrap();
    assert_ne!(first_image, second_image);

    // The old file is gone; the new one holds the new bytes.
    assert!(inv.image_path_if_exists(&first_image).is_none());
    let second_path = inv
        .image_path_if_exists(&second_image)
        .expect("second image on disk");
    assert_eq!(std::fs::read(&second_path).unwrap(), b"second");
}

#[tokio::test]
async fn set_image_on_missing_node_writes_nothing() {
    let (inv, _temp) = open_inventory().await;
    let err = inv.set_image("nope", b"x".to_vec()).await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn path_to_returns_root_first_breadcrumb() {
    let (inv, _temp) = open_inventory().await;

    let room = inv.create_child("root", "Room".into()).await.unwrap();
    let container = inv.create_child(&room.id, "Drawer".into()).await.unwrap();
    let thing = inv.create_child(&container.id, "Pen".into()).await.unwrap();

    let path = inv.path_to(&thing.id).await.unwrap();
    let ids: Vec<&str> = path.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "root",
            room.id.as_str(),
            container.id.as_str(),
            thing.id.as_str()
        ]
    );

    // The root's own breadcrumb is just itself.
    let root_path = inv.path_to("root").await.unwrap();
    assert_eq!(root_path.len(), 1);
    assert_eq!(root_path[0].id, "root");
}

#[tokio::test]
async fn path_to_missing_node_is_not_found() {
    let (inv, _temp) = open_inventory().await;
    let err = inv.path_to("nope").await.unwrap_err();
    assert!(
        matches!(err, visible_core::CoreError::NotFound(_)),
        "{err:?}"
    );
}

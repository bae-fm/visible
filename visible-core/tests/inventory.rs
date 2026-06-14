//! Tests the real [`Inventory`] against a real coven [`Database`] on a temp
//! directory — the production unit, not a reconstruction.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use coven::clock::{ClockRef, FixedClock};
use coven::id_provider::{IdRef, SequentialIdProvider};
use coven::library_dir::LibraryDir;
use coven::{Database, UpdatedAtStamper};
use tempfile::TempDir;
use visible_core::app::open_database;
use visible_core::node::insert_root;
use visible_core::Inventory;

/// Open a real database on a fresh temp library dir, lay down the schema and a
/// root house node, and return the live inventory (plus the tempdir, which must
/// outlive it). Mirrors what `library::create` + `app::bootstrap` do, so the
/// test drives the production node-tree code. Injects a deterministic id source
/// and a fixed clock so node/image ids and outbox timestamps are reproducible.
async fn open_inventory() -> (Inventory, TempDir) {
    let (inv, _db, temp) = open_inventory_with_db().await;
    (inv, temp)
}

/// Like [`open_inventory`] but also returns the database handle, so a test can
/// read coven's cloud outbox to assert the image upload/delete intents the node
/// writes enqueue.
async fn open_inventory_with_db() -> (Inventory, Database, TempDir) {
    let temp = TempDir::new().expect("temp dir");
    let dir = LibraryDir::new(temp.path().join("library"));
    std::fs::create_dir_all(&*dir).expect("create library dir");

    let (db, stamper) = open_database(&dir, "test-device".to_string()).expect("open database");

    seed_root(&db, &stamper).await;

    let ids: IdRef = Arc::new(SequentialIdProvider::new("node"));
    let clock: ClockRef = Arc::new(FixedClock(
        Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
    ));
    let inv = Inventory::new(db.clone(), stamper, dir, ids, clock);
    (inv, db, temp)
}

async fn seed_root(db: &Database, stamper: &UpdatedAtStamper) {
    let updated_at = stamper.stamp();
    db.call(move |conn| insert_root(conn, "root", "Home", &updated_at).map_err(Into::into))
        .await
        .expect("seed root");
}

/// Small stand-in image bytes for tests that need a child but don't care about
/// the image itself; every child is created photo-first.
const DUMMY_IMAGE: &[u8] = b"img";

#[tokio::test]
async fn root_exists_after_create() {
    let (inv, _temp) = open_inventory().await;
    let root = inv.root().await.expect("root");
    assert_eq!(root.id, "root");
    assert_eq!(root.name.as_deref(), Some("Home"));
    assert_eq!(root.parent_id, None);
    assert_eq!(root.position, 0);
}

#[tokio::test]
async fn children_keep_insertion_order() {
    let (inv, _temp) = open_inventory().await;

    let first = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let second = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let third = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

    assert_eq!(first.position, 0);
    assert_eq!(second.position, 1);
    assert_eq!(third.position, 2);

    let children = inv.children("root").await.unwrap();
    let ids: Vec<&str> = children.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![first.id.as_str(), second.id.as_str(), third.id.as_str()]
    );
    // A photo-first child is untitled until renamed.
    assert!(children.iter().all(|n| n.name.is_none()));
}

#[tokio::test]
async fn rename_titles_an_untitled_child() {
    let (inv, _temp) = open_inventory().await;
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    assert_eq!(node.name, None);

    inv.rename(&node.id, "Bookshelf".into()).await.unwrap();

    let reloaded = inv.get(&node.id).await.unwrap().unwrap();
    assert_eq!(reloaded.name.as_deref(), Some("Bookshelf"));
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
    let room = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let container = inv
        .create_child_with_image(&room.id, DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let thing = inv
        .create_child_with_image(&container.id, DUMMY_IMAGE.to_vec())
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
    let room = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

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
    let node = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

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
async fn create_child_with_image_lands_node_and_file_together() {
    let (inv, _temp) = open_inventory().await;

    let node = inv
        .create_child_with_image("root", b"toaster-photo".to_vec())
        .await
        .unwrap();

    // A photo-first child lands untitled.
    assert_eq!(node.name, None);
    assert_eq!(node.parent_id.as_deref(), Some("root"));
    let image_id = node.image_id.expect("node carries an image id");

    // The image file is on disk with the bytes we passed.
    let path = inv
        .image_path_if_exists(&image_id)
        .expect("image file on disk");
    assert_eq!(std::fs::read(&path).unwrap(), b"toaster-photo");

    // The node is a child of root, with its image id preserved on reload.
    let children = inv.children("root").await.unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, node.id);
    assert_eq!(children[0].image_id.as_deref(), Some(image_id.as_str()));
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

    let room = inv
        .create_child_with_image("root", DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let container = inv
        .create_child_with_image(&room.id, DUMMY_IMAGE.to_vec())
        .await
        .unwrap();
    let thing = inv
        .create_child_with_image(&container.id, DUMMY_IMAGE.to_vec())
        .await
        .unwrap();

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

/// The cloud key coven's blob layout and the outbox both use for an image:
/// `images/{ab}/{cd}/{image_id}`. The test builds it the same way the node write
/// does (via coven's `LibraryDir::hashed_path`) so it asserts the real key, not
/// a hand-spelled one.
fn image_cloud_key(image_id: &str) -> String {
    LibraryDir::hashed_path("images", image_id)
}

#[tokio::test]
async fn create_child_with_image_enqueues_the_upload_in_the_outbox() {
    let (inv, db, _temp) = open_inventory_with_db().await;

    let node = inv
        .create_child_with_image("root", b"photo".to_vec())
        .await
        .unwrap();
    let image_id = node.image_id.unwrap();

    let uploads = db.get_pending_cloud_uploads().await.unwrap();
    assert_eq!(uploads.len(), 1, "one upload queued for the new image");
    assert_eq!(uploads[0].cloud_key, image_cloud_key(&image_id));
    assert!(db.get_pending_cloud_deletes().await.unwrap().is_empty());
}

#[tokio::test]
async fn set_image_enqueues_new_upload_and_old_delete() {
    let (inv, db, _temp) = open_inventory_with_db().await;

    let node = inv
        .create_child_with_image("root", b"first".to_vec())
        .await
        .unwrap();
    let first_image = node.image_id.unwrap();

    inv.set_image(&node.id, b"second".to_vec()).await.unwrap();
    let second_image = inv.get(&node.id).await.unwrap().unwrap().image_id.unwrap();

    // Both images are queued for upload (the first from create, the second from
    // set_image); the replaced first image is queued for cloud deletion.
    let upload_keys: Vec<String> = db
        .get_pending_cloud_uploads()
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.cloud_key)
        .collect();
    assert!(upload_keys.contains(&image_cloud_key(&first_image)));
    assert!(upload_keys.contains(&image_cloud_key(&second_image)));

    let delete_keys: Vec<String> = db
        .get_pending_cloud_deletes()
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.cloud_key)
        .collect();
    assert_eq!(delete_keys, vec![image_cloud_key(&first_image)]);
}

#[tokio::test]
async fn delete_enqueues_a_cloud_delete_for_every_image_in_the_subtree() {
    let (inv, db, _temp) = open_inventory_with_db().await;

    let room = inv
        .create_child_with_image("root", b"room".to_vec())
        .await
        .unwrap();
    let thing = inv
        .create_child_with_image(&room.id, b"thing".to_vec())
        .await
        .unwrap();
    let room_image = room.image_id.unwrap();
    let thing_image = thing.image_id.unwrap();

    inv.delete(&room.id).await.unwrap();

    let delete_keys: Vec<String> = db
        .get_pending_cloud_deletes()
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.cloud_key)
        .collect();
    assert!(delete_keys.contains(&image_cloud_key(&room_image)));
    assert!(delete_keys.contains(&image_cloud_key(&thing_image)));
}
